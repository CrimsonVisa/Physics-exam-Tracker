import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { CalculatorPolicyPanel } from './CalculatorPolicyPanel';
import { getCalculatorProfile } from './profiles';

describe('CalculatorPolicyPanel', () => {
  it('renders the Practice mode without an examination policy', () => {
    render(<CalculatorPolicyPanel profile={getCalculatorProfile('practice')} />);
    expect(screen.getByRole('heading', { name: 'Practice mode' })).toBeInTheDocument();
    expect(screen.getByText(/not an examination simulation/)).toBeInTheDocument();
    expect(screen.getByText('Last verified: 2026-11-08')).toBeInTheDocument();
  });

  it('renders CSIR as prohibited physical and undocumented on-screen', () => {
    render(<CalculatorPolicyPanel profile={getCalculatorProfile('csir-net-physics')} />);
    expect(screen.getAllByText('Confirmed prohibited').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Not documented').length).toBeGreaterThan(0);
    expect(screen.getByText('June 2025')).toBeInTheDocument();
    expect(screen.getAllByText(/virtual keyboard/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/infomration-bulletin-of-csir-june-2025\.pdf/)).toBeInTheDocument();
  });

  it('renders GATE with the confirmed calculator and mock-reference caveat', () => {
    render(<CalculatorPolicyPanel profile={getCalculatorProfile('gate-physics')} />);
    expect(screen.getAllByText('Confirmed provided').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Not documented').length).toBeGreaterThan(0);
    expect(screen.getByText('GATE 2027')).toBeInTheDocument();
    expect(screen.getByText(/not an exact official GATE replica/i)).toBeInTheDocument();
    expect(screen.getByText(/keyboard operation is disabled/i)).toBeInTheDocument();
    expect(screen.getByText('GATE 2027 Information Brochure')).toBeInTheDocument();
    expect(screen.getByText('GATE 2026 Official Mock Calculator')).toBeInTheDocument();
  });

  it('renders TIFR with both GS 2024 evidence and GS 2027 verification caveat', () => {
    render(<CalculatorPolicyPanel profile={getCalculatorProfile('tifr-gs-physics')} />);
    expect(screen.getAllByText(/GS 2024/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/GS 2027/).length).toBeGreaterThan(0);
    expect(screen.getByText(/Function-level capabilities are not documented/i)).toBeInTheDocument();
    expect(screen.getAllByText('Confirmed provided').length).toBeGreaterThan(0);
  });

  it('renders JEST as prohibited physical and undocumented on-screen', () => {
    render(<CalculatorPolicyPanel profile={getCalculatorProfile('jest-physics')} />);
    expect(screen.getByText('JEST 2026')).toBeInTheDocument();
    expect(screen.getAllByText('Confirmed prohibited').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Not documented').length).toBeGreaterThan(0);
    expect(screen.getByText(/No on-screen calculator is documented/i)).toBeInTheDocument();
  });

  it('exposes official sources without translating undocumented into prohibited', () => {
    render(<CalculatorPolicyPanel profile={getCalculatorProfile('csir-net-physics')} />);
    expect(screen.getByText('On-screen calculator')).toBeInTheDocument();
    expect(screen.getByText('Function-level capabilities')).toBeInTheDocument();
    expect(screen.getByText('Official sources')).toBeInTheDocument();
    expect(screen.queryByText(/On-screen calculator.*prohibited/i)).not.toBeInTheDocument();
  });
});
