import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { ImportPreview, ImportResult } from '../lib/csvImportApi';
import { CsvImportPanel } from './CsvImportPanel';

const PREVIEW: ImportPreview = {
  totalRows: 3,
  validRows: 1,
  duplicateRows: 1,
  invalidRows: 1,
  rows: [
    {
      rowNumber: 2,
      status: 'valid',
      summary: '2024 · Part B · question 1',
      errors: [],
    },
    {
      rowNumber: 3,
      status: 'duplicate',
      summary: '2024 · Part B · question 2',
      errors: [{ field: 'duplicate', message: 'Already imported.' }],
    },
    {
      rowNumber: 4,
      status: 'invalid',
      summary: '2024 · Part C',
      errors: [{ field: 'attempt_date', message: 'Attempt date is invalid.' }],
    },
  ],
};

const RESULT: ImportResult = {
  imported: 1,
  skipped: 1,
  rejected: 1,
  rows: [
    {
      rowNumber: 2,
      status: 'imported',
      summary: '2024 · Part B · question 1',
      errors: [],
    },
    {
      rowNumber: 3,
      status: 'duplicate',
      summary: '2024 · Part B · question 2',
      errors: [{ field: 'duplicate', message: 'Already imported.' }],
    },
    {
      rowNumber: 4,
      status: 'invalid',
      summary: '2024 · Part C',
      errors: [{ field: 'attempt_date', message: 'Attempt date is invalid.' }],
    },
  ],
};

function props(overrides: Partial<Parameters<typeof CsvImportPanel>[0]> = {}) {
  return {
    id: 'test-import',
    title: 'Import test records',
    description: 'Use the template.',
    saveTemplate: vi.fn().mockResolvedValue('C:\\Users\\tester\\Downloads\\template.csv'),
    previewImport: vi.fn().mockResolvedValue(PREVIEW),
    applyImport: vi.fn().mockResolvedValue(RESULT),
    onImported: vi.fn(),
    ...overrides,
  };
}

async function chooseFile(text = 'year,part\n2024,B\n') {
  const file = new File([text], 'records.csv', { type: 'text/csv' });
  await userEvent.upload(screen.getByLabelText('CSV file'), file);
  expect(await screen.findByText('Selected: records.csv')).toBeInTheDocument();
}

describe('CSV import panel', () => {
  it('previews rows without writing and allows cancellation', async () => {
    const panelProps = props();

    render(<CsvImportPanel {...panelProps} />);

    await chooseFile();
    await userEvent.click(screen.getByRole('button', { name: 'Preview import' }));

    expect(await screen.findByText('Import 1 ready rows')).toBeInTheDocument();
    expect(screen.getByText(/Attempt date is invalid/)).toBeInTheDocument();
    expect(screen.getByText(/Already imported/)).toBeInTheDocument();
    expect(panelProps.previewImport).toHaveBeenCalledWith('year,part\n2024,B\n');
    expect(panelProps.applyImport).not.toHaveBeenCalled();

    await userEvent.click(screen.getAllByRole('button', { name: 'Cancel import' })[0]);

    expect(screen.queryByText('Ready')).not.toBeInTheDocument();
    expect(screen.queryByText('Selected: records.csv')).not.toBeInTheDocument();
    expect(panelProps.applyImport).not.toHaveBeenCalled();
  });

  it('imports ready rows and reports imported, skipped, and rejected counts', async () => {
    const panelProps = props();

    render(<CsvImportPanel {...panelProps} />);

    await chooseFile();
    await userEvent.click(screen.getByRole('button', { name: 'Preview import' }));
    await screen.findByText('Import 1 ready rows');
    await userEvent.click(screen.getByRole('button', { name: 'Import 1 ready rows' }));

    await waitFor(() => expect(panelProps.applyImport).toHaveBeenCalledWith('year,part\n2024,B\n'));
    expect(panelProps.onImported).toHaveBeenCalledWith(RESULT);
    expect(await screen.findByText('Start another import')).toBeInTheDocument();
    expect(screen.getByText('Attempt date is invalid.')).toBeInTheDocument();
  });

  it('shows backend header errors and disables import when no rows are ready', async () => {
    const previewImport = vi.fn().mockRejectedValue({
      code: 'validation_error',
      message: 'Missing required CSV columns: part.',
      field: 'file',
    });
    const panelProps = props({ previewImport });

    render(<CsvImportPanel {...panelProps} />);

    await chooseFile();
    await userEvent.click(screen.getByRole('button', { name: 'Preview import' }));

    expect(await screen.findByText('Missing required CSV columns: part.')).toBeInTheDocument();

    const emptyPreview: ImportPreview = {
      totalRows: 1,
      validRows: 0,
      duplicateRows: 0,
      invalidRows: 1,
      rows: [
        {
          rowNumber: 2,
          status: 'invalid',
          summary: '',
          errors: [{ field: 'part', message: 'Part must be A, B or C.' }],
        },
      ],
    };
    previewImport.mockResolvedValueOnce(emptyPreview);
    await userEvent.click(screen.getByRole('button', { name: 'Preview import' }));

    const importButton = await screen.findByRole('button', { name: 'Import 0 ready rows' });
    expect(importButton).toBeDisabled();
  });

  it('saves the header template to Downloads and reports the path', async () => {
    const panelProps = props();

    render(<CsvImportPanel {...panelProps} />);
    await userEvent.click(screen.getByRole('button', { name: 'Save template to Downloads' }));

    await waitFor(() => expect(panelProps.saveTemplate).toHaveBeenCalledTimes(1));
    expect(await screen.findByText(/Header-only template saved to/)).toBeInTheDocument();
    expect(screen.getByText('C:\\Users\\tester\\Downloads\\template.csv')).toBeInTheDocument();
  });

  it('rejects invalid UTF-8 text before previewing', async () => {
    const panelProps = props();

    render(<CsvImportPanel {...panelProps} />);
    const file = new File([new Uint8Array([0xff, 0xfe])], 'bad.csv', { type: 'text/csv' });
    await userEvent.upload(screen.getByLabelText('CSV file'), file);

    expect(await screen.findByText(/not valid UTF-8 text/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Preview import' })).toBeDisabled();
    expect(panelProps.previewImport).not.toHaveBeenCalled();
  });
});
