import logoIcon from '../assets/branding/physics-tracker-logo-icon.png';

type BrandMarkProps = {
  size?: number;
  className?: string;
};

/** Official Physics Tracker logo from the canonical branding source. */
export function BrandMark({ size = 36, className }: BrandMarkProps) {
  return (
    <span
      aria-hidden="true"
      className={`brand-mark-svg${className ? ` ${className}` : ''}`}
      style={{ width: size, height: size }}
    >
      <img alt="" draggable={false} src={logoIcon} />
    </span>
  );
}

type BrandLockupProps = {
  subtitle?: string;
};

/** Official logo plus the separately set application name. */
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
