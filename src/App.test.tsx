import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

describe('App scaffold', () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it('renders the application heading', () => {
    invokeMock.mockResolvedValue({ name: 'CSIR-NET Physics Tracker', version: '0.1.0' });

    render(<App />);

    expect(screen.getByRole('heading', { name: 'CSIR-NET Physics Tracker' })).toBeInTheDocument();
  });

  it('reports a connected backend when the Rust command succeeds', async () => {
    invokeMock.mockResolvedValue({ name: 'CSIR-NET Physics Tracker', version: '0.1.0' });

    render(<App />);

    expect(await screen.findByText(/Tauri backend connected/i)).toBeInTheDocument();
  });

  it('explains how to connect the backend when running outside Tauri', async () => {
    invokeMock.mockRejectedValue(new Error('not running in Tauri'));

    render(<App />);

    expect(await screen.findByText(/Tauri backend unavailable/i)).toBeInTheDocument();
  });
});
