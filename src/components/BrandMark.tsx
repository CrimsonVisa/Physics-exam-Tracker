const P_PATH =
  'M40 72H176A88 88 0 0 1 176 248H96V440H40V72ZM96 128H176A32 32 0 0 1 176 192H96V128Z';
const H_PATH = 'M296 72H352V228H416V72H472V440H416V284H352V440H296V72Z';

type BrandMarkProps = {
  size?: number;
  className?: string;
};

/**
 * The PH-only lettermark. The tile and glyph are themed through CSS custom
 * properties so the same accessible SVG works on dark and light surfaces.
 */
export function BrandMark({ size = 36, className }: BrandMarkProps) {
  return (
    <span
      aria-hidden="true"
      className={`brand-mark-svg${className ? ` ${className}` : ''}`}
      style={{ width: size, height: size }}
    >
      <svg focusable="false" viewBox="0 0 512 512">
        <rect className="brand-tile" height="512" rx="116" width="512" />
        <g className="brand-glyph" transform="translate(87 87) scale(0.66)">
          <path d={P_PATH} fillRule="evenodd" />
          <path d={H_PATH} />
        </g>
      </svg>
    </span>
  );
}

type BrandLockupProps = {
  subtitle?: string;
};

/** Brand mark plus the separately set application name (the logo stays PH-only). */
export function BrandLockup({ subtitle = 'Four-exam foundation' }: BrandLockupProps) {
  return (
    <div className="v2-boot-brand">
      <BrandMark size={48} />
      <div>
        <p className="brand-title">Physics Tracker</p>
        <p className="brand-subtitle">{subtitle}</p>
      </div>
    </div>
  );
}
