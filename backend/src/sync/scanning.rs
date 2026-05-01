use std::{collections::HashMap, fs};

use super::{
    operation_file::{
        corrupt_without_content, is_temp_operation_file_name, parse_operation_file_name,
        read_operation_file_for_identity,
    },
    CorruptRemoteReason, OperationScanReport, SharedSyncPaths, SyncCoreResult, OP_FILE_SUFFIX,
};

pub(crate) fn scan_operation_files(paths: &SharedSyncPaths) -> SyncCoreResult<OperationScanReport> {
    scan_operation_files_after_watermarks(paths, &HashMap::new())
}

pub(crate) fn scan_operation_files_after_watermarks(
    paths: &SharedSyncPaths,
    watermarks: &HashMap<String, u64>,
) -> SyncCoreResult<OperationScanReport> {
    let mut report = OperationScanReport::default();
    if !paths.ops_dir.exists() {
        return Ok(report);
    }

    let mut seen_sequences: HashMap<(String, u64), String> = HashMap::new();

    for client_dir in fs::read_dir(&paths.ops_dir)? {
        let client_dir = client_dir?;
        if !client_dir.file_type()?.is_dir() {
            report.ignored_unknown_files += 1;
            continue;
        }

        let client_path = client_dir.path();
        let expected_client_id = client_dir.file_name().to_string_lossy().into_owned();

        for file in fs::read_dir(&client_path)? {
            let file = file?;
            if !file.file_type()?.is_file() {
                report.ignored_unknown_files += 1;
                continue;
            }

            let path = file.path();
            let file_name = file.file_name().to_string_lossy().into_owned();
            if is_temp_operation_file_name(&file_name) {
                report.ignored_temp_files += 1;
                continue;
            }

            if !file_name.ends_with(OP_FILE_SUFFIX) {
                report.ignored_unknown_files += 1;
                continue;
            }

            let expected_seq = match parse_operation_file_name(&file_name) {
                Ok(seq) => seq,
                Err(detail) => {
                    report.corrupt.push(corrupt_without_content(
                        &path,
                        CorruptRemoteReason::InvalidFileName,
                        detail,
                    ));
                    continue;
                }
            };

            if watermarks
                .get(&expected_client_id)
                .is_some_and(|watermark| expected_seq <= *watermark)
            {
                report.ignored_watermarked_files += 1;
                continue;
            }

            match read_operation_file_for_identity(&path, &expected_client_id, expected_seq) {
                Ok(operation) => {
                    let sequence_key = (operation.client_id.clone(), operation.local_seq);
                    match seen_sequences.get(&sequence_key) {
                        Some(checksum) if checksum != &operation.checksum => {
                            report.corrupt.push(corrupt_without_content(
                                &path,
                                CorruptRemoteReason::DuplicateSequenceDifferentChecksum,
                                "Duplicate client_id and local_seq has different content.",
                            ));
                        }
                        Some(_) => {}
                        None => {
                            seen_sequences.insert(sequence_key, operation.checksum.clone());
                            report.operations.push(operation);
                        }
                    }
                }
                Err(corrupt) => report.corrupt.push(corrupt),
            }
        }
    }

    report.operations.sort_by(|left, right| {
        left.client_id
            .cmp(&right.client_id)
            .then_with(|| left.local_seq.cmp(&right.local_seq))
            .then_with(|| left.op_id.cmp(&right.op_id))
    });

    Ok(report)
}
