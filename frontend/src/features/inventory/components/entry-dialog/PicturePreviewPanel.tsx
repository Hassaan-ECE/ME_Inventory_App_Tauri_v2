import { PicturePreviewCard } from "./components";
import type { EntryPicturePreviewControls } from "./useEntryPicturePreview";

interface PicturePreviewPanelProps {
  compact?: boolean;
  picturePath: string;
  preview: EntryPicturePreviewControls;
}

export function PicturePreviewPanel({ compact = false, picturePath, preview }: PicturePreviewPanelProps) {
  return (
    <PicturePreviewCard
      canBrowse={preview.canBrowsePicture}
      canOpen={preview.canOpenPicture}
      compact={compact}
      picturePath={picturePath}
      previewSrc={preview.picturePreviewSrc}
      previewState={preview.picturePreviewState}
      onBrowse={() => {
        void preview.handleBrowsePicture();
      }}
      onOpen={() => {
        void preview.handleOpenPicture();
      }}
      onPreviewError={preview.handlePreviewError}
      onPreviewLoad={preview.handlePreviewLoad}
    />
  );
}
