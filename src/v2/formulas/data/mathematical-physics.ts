import type { FormulaEntry } from '../types';
import { CSIR, GATE, JEST, TIFR, link } from './syllabus';
import { REFERENCES, TEXTBOOKS, reference, textbook } from './sources';

export const MATHEMATICAL_PHYSICS_FORMULAS: FormulaEntry[] = [
  {
    id: 'mathematical-physics-divergence-theorem',
    title: 'Divergence theorem',
    latex: String.raw`\iiint_V (\nabla \cdot \mathbf{F})\,dV = \iint_{\partial V} \mathbf{F}\cdot d\mathbf{S}`,
    plainText: 'volume integral div F dV = surface integral F dot dS',
    subject: 'mathematical-physics',
    topic: 'Vector calculus',
    subtopic: 'Integral theorems',
    summary:
      'Relates the divergence of a vector field inside a volume to the outward flux through its closed boundary surface.',
    symbols: [
      {
        symbol: String.raw`\mathbf{F}`,
        meaning: 'continuously differentiable vector field',
      },
      { symbol: 'V', meaning: 'bounded volume region' },
      { symbol: String.raw`\partial V`, meaning: 'closed boundary surface of V' },
      {
        symbol: String.raw`d\mathbf{S}`,
        meaning: 'outward vector area element',
        dimensions: 'L^2',
      },
    ],
    conditions: [
      'F is continuously differentiable throughout the volume.',
      'V is bounded with a piecewise smooth closed boundary.',
      'The area element points outward from V.',
    ],
    exactness: 'exact',
    framework: 'general',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.divergence]),
      link('gate-physics', 'implicit', [GATE.math]),
      link('tifr-gs-physics', 'implicit', [TIFR.math]),
      link('jest-physics', 'explicit', [JEST.vectorCalculus]),
    ],
    sources: [textbook(TEXTBOOKS.arfken, 'Ch. 3'), textbook(TEXTBOOKS.riley, 'Ch. 11')],
    relatedIds: ['electromagnetism-gauss-law-differential'],
    conventions: ['Right-hand orientation; dS is the outward normal area element.'],
    keywords: ['gauss theorem', 'flux', 'divergence', 'vector calculus', '∇'],
  },
  {
    id: 'mathematical-physics-cauchy-integral-formula',
    title: 'Cauchy integral formula',
    latex: String.raw`f(z_0)=\frac{1}{2\pi i}\oint_C\frac{f(z)}{z-z_0}\,dz`,
    plainText: 'f(z0) = (1/(2 pi i)) contour integral f(z)/(z-z0) dz',
    subject: 'mathematical-physics',
    topic: 'Complex analysis',
    subtopic: 'Cauchy integral formula',
    summary:
      'Recovers the value of an analytic function at an interior point from its values on a positively oriented closed contour.',
    symbols: [
      { symbol: 'f', meaning: 'function analytic on and inside C' },
      { symbol: String.raw`z_0`, meaning: 'interior point of C' },
      { symbol: 'C', meaning: 'simple closed piecewise-smooth contour' },
      { symbol: 'i', meaning: 'imaginary unit' },
    ],
    conditions: [
      'f is analytic on and inside C.',
      'C is a simple closed piecewise-smooth contour.',
      'z0 lies inside C.',
      'C is traversed in the positive counter-clockwise orientation.',
    ],
    exactness: 'exact',
    framework: 'general',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.cauchy]),
      link('gate-physics', 'explicit', [GATE.complexAnalysis]),
      link('tifr-gs-physics', 'implicit', [TIFR.math]),
      link('jest-physics', 'explicit', [JEST.complexAnalysis]),
    ],
    sources: [textbook(TEXTBOOKS.arfken, 'Ch. 11'), textbook(TEXTBOOKS.riley, 'Ch. 24')],
    relatedIds: ['mathematical-physics-fourier-transform'],
    conventions: ['Positive orientation means counter-clockwise traversal.'],
    keywords: ['complex analysis', 'contour integral', 'residue', 'analytic function'],
  },
  {
    id: 'mathematical-physics-fourier-transform',
    title: 'Fourier transform pair (unitary angular-wavenumber/angular-frequency convention)',
    latex: String.raw`F(k)=\frac{1}{\sqrt{2\pi}}\int_{-\infty}^{\infty}f(x)e^{-ikx}\,dx,\qquad f(x)=\frac{1}{\sqrt{2\pi}}\int_{-\infty}^{\infty}F(k)e^{ikx}\,dk`,
    plainText:
      'F(k) = (1/sqrt(2 pi)) integral f(x) exp(-i k x) dx; f(x) = (1/sqrt(2 pi)) integral F(k) exp(i k x) dk',
    subject: 'mathematical-physics',
    topic: 'Fourier analysis',
    subtopic: 'Fourier transform',
    summary:
      'The forward and inverse Fourier transforms in the unitary convention with conjugate variables x and k; k is the angular wavenumber when x is spatial and the angular frequency when x is temporal.',
    symbols: [
      { symbol: 'f', meaning: 'function being transformed' },
      { symbol: 'F', meaning: 'Fourier transform of f' },
      {
        symbol: 'k',
        meaning:
          'angular wavenumber when x is a spatial coordinate, or angular frequency when x is time',
      },
      { symbol: 'x', meaning: 'spatial coordinate or time' },
    ],
    conditions: [
      'f is absolutely integrable on the real line, or the transform is interpreted in a distributional sense.',
      'The forward and inverse transforms both use the unitary convention shown.',
      'When x is position, k is the angular wavenumber; when x is time, k is the angular frequency.',
      'The inverse transform uses +ikx with the same 1/sqrt(2 pi) factor.',
    ],
    exactness: 'definition',
    framework: 'general',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.fourier]),
      link('gate-physics', 'explicit', [GATE.fourierAnalysis]),
      link('tifr-gs-physics', 'implicit', [TIFR.math]),
      link('jest-physics', 'explicit', [JEST.fourierTransforms]),
    ],
    sources: [
      textbook(TEXTBOOKS.arfken, 'Ch. 20'),
      textbook(TEXTBOOKS.riley, 'Ch. 13'),
      reference(REFERENCES.dlmf, 'Fourier and related transforms'),
    ],
    relatedIds: ['mathematical-physics-green-function-poisson'],
    conventions: [
      'Unitary conjugate-variable convention: f(x) = (1/sqrt(2 pi)) integral F(k) exp(ikx) dk.',
      'When x is position, k is the angular wavenumber; when x is time, k is the angular frequency.',
      'Other conventions use 1/(2 pi) or 1 factors; they must not be mixed.',
    ],
    keywords: [
      'fourier',
      'transform',
      'convolution',
      'spectrum',
      'angular wavenumber',
      'angular frequency',
      'ω',
    ],
  },
  {
    id: 'mathematical-physics-green-function-poisson',
    title: "Green's function for the Poisson equation",
    latex: String.raw`\nabla^2 G(\mathbf{r},\mathbf{r}')=\delta^{(3)}(\mathbf{r}-\mathbf{r}')`,
    plainText: "laplacian G(r,r') = delta^3(r-r')",
    subject: 'mathematical-physics',
    topic: 'Partial differential equations',
    subtopic: 'Green functions',
    summary:
      'Defines the fundamental response of the Poisson operator to a unit point source; boundary conditions select the physical Green function.',
    symbols: [
      { symbol: 'G', meaning: 'Green function for the Poisson operator' },
      { symbol: String.raw`\mathbf{r}`, meaning: 'field point' },
      { symbol: String.raw`\mathbf{r}'`, meaning: 'source point' },
      {
        symbol: String.raw`\delta^{(3)}`,
        meaning: 'three-dimensional Dirac delta distribution',
        dimensions: 'L^-3',
      },
    ],
    conditions: [
      'The equation is interpreted in the distributional sense.',
      'The sign convention is laplacian G = +delta^3; some texts use the opposite sign.',
      'A domain and homogeneous boundary conditions must be specified.',
      'A solution of the homogeneous equation may be added to satisfy boundary conditions.',
    ],
    exactness: 'definition',
    framework: 'general',
    syllabusLinks: [
      link('csir-net-physics', 'explicit', [CSIR.green]),
      link('gate-physics', 'implicit', [GATE.math]),
      link('tifr-gs-physics', 'implicit', [TIFR.math]),
      link('jest-physics', 'implicit', [JEST.math]),
    ],
    sources: [textbook(TEXTBOOKS.arfken, 'Ch. 10'), textbook(TEXTBOOKS.riley, 'Ch. 23')],
    relatedIds: ['mathematical-physics-fourier-transform'],
    conventions: [
      'Boundary conditions are part of the definition of the physical Green function.',
      'The displayed sign convention is laplacian G = +delta^3.',
    ],
    keywords: ['green function', 'poisson', 'laplace', 'point source', 'pde'],
  },
];
