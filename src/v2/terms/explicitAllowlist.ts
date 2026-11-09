import { CSIR, GATE, JEST } from './data/shared';

/**
 * Explicit syllabus links are a manual scientific claim: the node title must
 * name the term itself or an exact, unambiguous synonym. Because the
 * structured syllabus only exposes titles, this reviewed allowlist makes the
 * claim explicit and testable. Any new explicit link must be reviewed here.
 */
export function explicitLinkKey(entryId: string, examCode: string, nodeCode: string): string {
  return `${entryId}|${examCode}|${nodeCode}`;
}

export const EXPLICIT_TERM_LINKS = new Set([
  explicitLinkKey('mathematical-physics-fourier-transform', 'csir-net-physics', CSIR.fourier.code),
  explicitLinkKey(
    'mathematical-physics-fourier-transform',
    'jest-physics',
    JEST.fourierTransforms.code,
  ),
  explicitLinkKey('classical-mechanics-momentum', 'jest-physics', JEST.newton.code),
  explicitLinkKey('units-dimension', 'gate-physics', GATE.unitsDimensions.code),
  explicitLinkKey('mathematical-physics-eigenvalue', 'csir-net-physics', CSIR.eigenvalue.code),
  explicitLinkKey('condensed-matter-effective-mass', 'gate-physics', GATE.conductivity.code),
  explicitLinkKey('optics-coherence', 'csir-net-physics', CSIR.diffraction.code),
  explicitLinkKey('nuclear-particle-binding-energy', 'csir-net-physics', CSIR.semiEmpirical.code),
  explicitLinkKey('quantum-mechanics-commutator', 'csir-net-physics', CSIR.commutator.code),
  explicitLinkKey(
    'quantum-mechanics-uncertainty-principle',
    'csir-net-physics',
    CSIR.commutator.code,
  ),
  explicitLinkKey('quantum-mechanics-uncertainty-principle', 'jest-physics', JEST.schrodinger.code),
  explicitLinkKey('thermodynamics-partition-function', 'csir-net-physics', CSIR.partition.code),
  explicitLinkKey('thermodynamics-partition-function', 'gate-physics', GATE.partition.code),
  explicitLinkKey('thermodynamics-partition-function', 'jest-physics', JEST.partition.code),
  explicitLinkKey('experimental-methods-chi-square', 'csir-net-physics', CSIR.chiSquare.code),
  explicitLinkKey('optics-polarization', 'csir-net-physics', CSIR.polarization.code),
  explicitLinkKey('optics-polarization', 'gate-physics', GATE.polarization.code),
  explicitLinkKey('optics-polarization', 'jest-physics', JEST.optics.code),
  explicitLinkKey('waves-oscillations-group-velocity', 'gate-physics', GATE.waves.code),
  explicitLinkKey('electromagnetism-poynting-vector', 'gate-physics', GATE.poynting.code),
]);
