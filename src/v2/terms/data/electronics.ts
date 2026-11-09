import type { TermEntry } from '../types';
import { CSIR, GATE, JEST, TEXTBOOKS, TIFR, link, textbook } from './shared';

const opAmpLinks = () => [
  link('csir-net-physics', 'implicit', [CSIR.operationalAmplifiers]),
  link('gate-physics', 'implicit', [GATE.opAmps]),
  link('tifr-gs-physics', 'implicit', [TIFR.electronics]),
  link('jest-physics', 'implicit', [JEST.opAmps]),
];

const virtualGroundLinks = () => [
  link('csir-net-physics', 'implicit', [CSIR.operationalAmplifiers]),
  link('gate-physics', 'implicit', [GATE.opAmps]),
  link('tifr-gs-physics', 'implicit', [TIFR.electronics]),
  link('jest-physics', 'implicit', [JEST.opAmps]),
];

const electronicsLinks = () => [
  link('csir-net-physics', 'implicit', [CSIR.electronicsCore]),
  link('gate-physics', 'implicit', [GATE.electronics]),
  link('tifr-gs-physics', 'implicit', [TIFR.electronics]),
  link('jest-physics', 'implicit', [JEST.electronics]),
];

export const ELECTRONICS_TERMS: TermEntry[] = [
  {
    id: 'electronics-ideal-opamp',
    term: 'ideal operational amplifier',
    aliases: ['ideal op-amp'],
    kind: 'concept',
    subject: 'electronics',
    topic: 'Operational amplifiers',
    definition:
      'An idealized differential amplifier with infinite open-loop gain, infinite input impedance and zero output impedance.',
    conditions: [
      'The ideal model is used with negative feedback and operation in the linear region.',
      'Real op-amps have finite gain, finite input impedance, nonzero output impedance, offset and bandwidth limits.',
    ],
    relatedTermIds: [
      'electronics-virtual-ground',
      'electronics-negative-feedback',
      'electronics-input-impedance',
    ],
    relatedFormulaIds: ['electronics-inverting-opamp-gain'],
    syllabusLinks: opAmpLinks(),
    sources: [textbook(TEXTBOOKS.sedraSmith, 'Ch. 2'), textbook(TEXTBOOKS.horowitzHill, 'Ch. 4')],
    keywords: ['ideal op-amp', 'operational amplifier', 'infinite gain', 'input impedance'],
  },
  {
    id: 'electronics-virtual-ground',
    term: 'virtual ground',
    kind: 'concept',
    subject: 'electronics',
    topic: 'Operational amplifiers',
    definition:
      'A circuit node held at approximately zero potential without a physical ground connection; in the inverting op-amp configuration with negative feedback, the inverting input is a virtual ground.',
    conditions: [
      'This description applies to the inverting configuration; in a non-inverting configuration the inverting input is held at the input signal potential, not ground.',
      'The virtual-ground approximation relies on high open-loop gain and negative feedback.',
      'The node is not a true ground and cannot sink arbitrary current.',
    ],
    relatedTermIds: ['electronics-ideal-opamp', 'electronics-negative-feedback'],
    relatedFormulaIds: ['electronics-inverting-opamp-gain'],
    syllabusLinks: virtualGroundLinks(),
    sources: [textbook(TEXTBOOKS.sedraSmith, 'Ch. 2'), textbook(TEXTBOOKS.horowitzHill, 'Ch. 4')],
    keywords: ['virtual ground', 'op-amp', 'inverting input', 'negative feedback'],
  },
  {
    id: 'electronics-negative-feedback',
    term: 'negative feedback',
    kind: 'concept',
    subject: 'electronics',
    topic: 'Feedback',
    definition:
      'A feedback connection in which a fraction of the output is returned to the input with a phase that opposes the input signal.',
    conditions: [
      'Negative feedback can stabilize gain, modify input and output impedance and extend bandwidth.',
      'Excessive phase shift can turn negative feedback into oscillation.',
    ],
    relatedTermIds: ['electronics-ideal-opamp', 'electronics-virtual-ground'],
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.operationalAmplifiers]),
      link('gate-physics', 'implicit', [GATE.opAmps]),
      link('tifr-gs-physics', 'implicit', [TIFR.electronics]),
      link('jest-physics', 'implicit', [JEST.opAmps]),
    ],
    sources: [textbook(TEXTBOOKS.sedraSmith, 'Ch. 2'), textbook(TEXTBOOKS.horowitzHill, 'Ch. 4')],
    keywords: ['negative feedback', 'feedback', 'gain stability', 'oscillation'],
  },
  {
    id: 'electronics-input-impedance',
    term: 'input impedance',
    kind: 'quantity',
    subject: 'electronics',
    topic: 'Circuit impedance',
    definition: 'The impedance presented by a circuit or device at its input port.',
    symbol: String.raw`Z_{\mathrm{in}}`,
    symbolPlainText: 'Z_in',
    siUnit: 'Ω',
    siUnitLatex: String.raw`\Omega`,
    dimensions: 'M L^2 T^-3 I^-2',
    dimensionFormula: String.raw`[Z_{\mathrm{in}}]=M L^{2} T^{-3} I^{-2}`,
    conditions: [
      'Input impedance is frequency-dependent in reactive circuits.',
      'For an inverting op-amp circuit the input impedance is approximately the external input resistor, not the op-amp input impedance.',
    ],
    relatedTermIds: ['electronics-output-impedance', 'electronics-ideal-opamp', 'units-ohm'],
    syllabusLinks: electronicsLinks(),
    sources: [textbook(TEXTBOOKS.sedraSmith, 'Ch. 1'), textbook(TEXTBOOKS.horowitzHill, 'Ch. 1')],
    keywords: ['input impedance', 'Zin', 'input resistance', 'op-amp'],
  },
  {
    id: 'electronics-output-impedance',
    term: 'output impedance',
    kind: 'quantity',
    subject: 'electronics',
    topic: 'Circuit impedance',
    definition: 'The impedance presented by a circuit or device at its output port.',
    symbol: String.raw`Z_{\mathrm{out}}`,
    symbolPlainText: 'Z_out',
    siUnit: 'Ω',
    siUnitLatex: String.raw`\Omega`,
    dimensions: 'M L^2 T^-3 I^-2',
    dimensionFormula: String.raw`[Z_{\mathrm{out}}]=M L^{2} T^{-3} I^{-2}`,
    conditions: [
      'Output impedance is frequency-dependent in reactive circuits.',
      'Negative feedback generally lowers the output impedance of an op-amp circuit.',
    ],
    relatedTermIds: ['electronics-input-impedance', 'electronics-ideal-opamp', 'units-ohm'],
    syllabusLinks: electronicsLinks(),
    sources: [textbook(TEXTBOOKS.sedraSmith, 'Ch. 1'), textbook(TEXTBOOKS.horowitzHill, 'Ch. 1')],
    keywords: ['output impedance', 'Zout', 'output resistance', 'op-amp'],
  },
  {
    id: 'electronics-thermal-voltage',
    term: 'thermal voltage',
    kind: 'quantity',
    subject: 'electronics',
    topic: 'Semiconductor devices',
    definition:
      'The voltage scale k_B T/q that appears in the diode equation and other carrier-transport expressions.',
    symbol: String.raw`V_T`,
    symbolPlainText: 'V_T',
    siUnit: 'V',
    siUnitLatex: String.raw`\mathrm{V}`,
    dimensions: 'M L^2 T^-3 I^-1',
    dimensionFormula: String.raw`[V_T]=M L^{2} T^{-3} I^{-1}`,
    conditions: [
      'At room temperature the thermal voltage is approximately 26 mV, but the value must be evaluated at the stated temperature.',
      'The symbol q denotes the magnitude of the elementary charge in this convention.',
    ],
    relatedTermIds: ['units-volt'],
    relatedFormulaIds: ['electronics-diode-equation'],
    syllabusLinks: [
      link('csir-net-physics', 'implicit', [CSIR.semiconductorDevices]),
      link('gate-physics', 'implicit', [GATE.pnDiodes]),
      link('tifr-gs-physics', 'implicit', [TIFR.electronics]),
      link('jest-physics', 'implicit', [JEST.pnJunctions]),
    ],
    sources: [textbook(TEXTBOOKS.sedraSmith, 'Ch. 4'), textbook(TEXTBOOKS.horowitzHill, 'Ch. 1')],
    keywords: ['thermal voltage', 'diode', 'kT/q', 'semiconductor'],
  },
];
