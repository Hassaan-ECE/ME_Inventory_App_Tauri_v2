/* @vitest-environment jsdom */
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it } from "vitest";

import { InventoryTable } from "@/components/inventory/InventoryTable";
import { MOCK_INVENTORY } from "@/data/mockInventory";
import { DEFAULT_FILTERS, filterEntries, sortEntries } from "@/lib/inventory";
import { INVENTORY_COLUMNS, type InventoryEntry } from "@/types/inventory";

interface DurationMetric {
  dataset: string;
  entries: number;
  iterations: number;
  max: number;
  median: number;
  min: number;
  operation: string;
  p95: number;
  unit: "ms";
}

interface MemoryMetric {
  dataset: string;
  entries: number;
  operation: string;
  unit: "mb";
  value: number;
}

type Metric = DurationMetric | MemoryMetric;

describe.runIf(process.env.RUN_PERF_BASELINE === "1")("frontend performance baseline", () => {
  it("measures local filtering and virtualized table behavior", async () => {
    globalThis.IS_REACT_ACT_ENVIRONMENT = true;
    const datasets = [
      { entries: MOCK_INVENTORY, name: "current" },
      { entries: syntheticEntries(1_000), name: "synthetic_1000" },
      { entries: syntheticEntries(10_000), name: "synthetic_10000" },
    ] as const;
    const metrics: Metric[] = [];

    for (const dataset of datasets) {
      metrics.push(
        measureDuration(dataset.name, dataset.entries, "local_filter_sort", 35, () => {
          const filtered = filterEntries(dataset.entries, "inventory", "calibration", DEFAULT_FILTERS);
          const sorted = sortEntries(filtered, { column: "manufacturer", direction: "asc" });
          void sorted.length;
        }),
      );

      metrics.push(await measureTableRender(dataset.name, dataset.entries));
      metrics.push(await measureTableScroll(dataset.name, dataset.entries));
      metrics.push({
        dataset: dataset.name,
        entries: dataset.entries.length,
        operation: "process_heap_used_after_dataset",
        unit: "mb",
        value: round(process.memoryUsage().heapUsed / 1024 / 1024),
      });
    }

    const outputPath = resolve(process.env.PERF_BASELINE_JSON ?? ".tmp/performance-baseline-frontend.json");
    await mkdir(dirname(outputPath), { recursive: true });
    await writeFile(outputPath, `${JSON.stringify(metrics, null, 2)}\n`);

    console.table(
      metrics.map((metric) =>
        metric.unit === "ms"
          ? {
              dataset: metric.dataset,
              entries: metric.entries,
              max: metric.max,
              median: metric.median,
              operation: metric.operation,
              p95: metric.p95,
              unit: metric.unit,
            }
          : {
              dataset: metric.dataset,
              entries: metric.entries,
              operation: metric.operation,
              unit: metric.unit,
              value: metric.value,
            },
      ),
    );
    console.log(`Wrote frontend performance baseline to ${outputPath}`);

    expect(metrics.length).toBeGreaterThan(0);
  });
});

function measureDuration(
  dataset: string,
  entries: readonly InventoryEntry[],
  operation: string,
  iterations: number,
  run: () => void,
): DurationMetric {
  const samples: number[] = [];
  for (let index = 0; index < iterations; index += 1) {
    const started = performance.now();
    run();
    samples.push(performance.now() - started);
  }

  return durationMetric(dataset, entries.length, operation, samples);
}

async function measureTableRender(dataset: string, entries: readonly InventoryEntry[]): Promise<DurationMetric> {
  const samples: number[] = [];

  for (let index = 0; index < 15; index += 1) {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    const started = performance.now();

    await act(async () => {
      root.render(<BenchmarkTable entries={entries} />);
    });

    samples.push(performance.now() - started);
    root.unmount();
    container.remove();
  }

  return durationMetric(dataset, entries.length, "inventory_table_initial_render", samples);
}

async function measureTableScroll(dataset: string, entries: readonly InventoryEntry[]): Promise<DurationMetric> {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);

  await act(async () => {
    root.render(<BenchmarkTable entries={entries} />);
  });

  const scroller = container.querySelector("section > div") as HTMLDivElement | null;
  const samples: number[] = [];

  for (let index = 0; index < 25; index += 1) {
    const started = performance.now();
    await act(async () => {
      if (scroller) {
        scroller.scrollTop = Math.min((index + 1) * 450, entries.length * 45);
        scroller.dispatchEvent(new window.Event("scroll", { bubbles: true }));
      }
    });
    samples.push(performance.now() - started);
  }

  root.unmount();
  container.remove();
  return durationMetric(dataset, entries.length, "inventory_table_scroll_update", samples);
}

function BenchmarkTable({ entries }: { entries: readonly InventoryEntry[] }) {
  return (
    <InventoryTable
      canModifyEntries
      colorRows
      columns={INVENTORY_COLUMNS}
      entries={[...entries]}
      sortState={{ column: "manufacturer", direction: "asc" }}
      onOpenContextMenu={() => undefined}
      onOpenEntry={() => undefined}
      onOpenExternalLink={() => undefined}
      onSortChange={() => undefined}
      onToggleVerified={() => undefined}
    />
  );
}

function durationMetric(
  dataset: string,
  entries: number,
  operation: string,
  samples: number[],
): DurationMetric {
  samples.sort((left, right) => left - right);
  const lastIndex = samples.length - 1;
  const p95Index = Math.min(lastIndex, Math.ceil(lastIndex * 0.95));

  return {
    dataset,
    entries,
    iterations: samples.length,
    max: round(samples[lastIndex]),
    median: round(samples[Math.floor(samples.length / 2)]),
    min: round(samples[0]),
    operation,
    p95: round(samples[p95Index]),
    unit: "ms",
  };
}

function syntheticEntries(size: number): InventoryEntry[] {
  return Array.from({ length: size }, (_, index) => {
    const base = MOCK_INVENTORY[index % MOCK_INVENTORY.length];
    const id = index + 1;

    return {
      ...base,
      archived: index % 10 === 0,
      assetNumber: `ME-${String(id).padStart(5, "0")}`,
      assignedTo: `User ${index % 19}`,
      condition: index % 7 === 0 ? "Calibration due" : "Good",
      description: `Calibration fixture and measurement asset ${id}`,
      id: `perf-${id}`,
      location: `Bay ${index % 16}`,
      manufacturer: `Maker ${String(index % 37).padStart(2, "0")}`,
      model: `Model ${index % 113}`,
      notes: `Synthetic performance note ${id} with calibration history`,
      projectName: `Project ${index % 29}`,
      qty: index % 11 === 0 ? null : (index % 23) + 1,
      serialNumber: `SN-${String(id).padStart(5, "0")}`,
      updatedAt: `2026-04-${String((index % 28) + 1).padStart(2, "0")}T12:00:00.000Z`,
      verifiedInSurvey: index % 3 === 0,
    };
  });
}

function round(value: number): number {
  return Math.round(value * 1_000) / 1_000;
}
