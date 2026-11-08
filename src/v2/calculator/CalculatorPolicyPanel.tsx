import { evidenceStatusLabel } from './profiles';
import type { CalculatorProfile, EvidenceStatus } from './types';

type CalculatorPolicyPanelProps = {
  profile: CalculatorProfile;
};

function statusTone(status: EvidenceStatus): string {
  switch (status) {
    case 'confirmed-prohibited':
      return 'prohibited';
    case 'confirmed-provided':
    case 'confirmed-permitted':
      return 'confirmed';
    case 'year-dependent':
      return 'dependent';
    case 'not-documented':
    case 'not-provided':
    default:
      return 'undocumented';
  }
}

function PolicyStatus({ status }: { status: EvidenceStatus }) {
  return (
    <span className="v2-calculator-policy-status" data-tone={statusTone(status)}>
      {evidenceStatusLabel(status)}
    </span>
  );
}

export function CalculatorPolicyPanel({ profile }: CalculatorPolicyPanelProps) {
  if (profile.modeType === 'practice') {
    return (
      <section
        aria-labelledby="calculator-policy-heading"
        className="v2-concept-card v2-calculator-policy"
      >
        <p className="eyebrow">Calculator policy</p>
        <h2 id="calculator-policy-heading">Practice mode</h2>
        <p className="v2-calculator-policy-summary">{profile.summary}</p>
        <p className="v2-calculator-policy-meta">Last verified: {profile.lastVerified}</p>
      </section>
    );
  }

  return (
    <section
      aria-labelledby="calculator-policy-heading"
      className="v2-concept-card v2-calculator-policy"
    >
      <div className="v2-calculator-section-heading">
        <div>
          <p className="eyebrow">Examination policy</p>
          <h2 id="calculator-policy-heading">Calculator policy</h2>
        </div>
      </div>

      <p className="v2-calculator-policy-meta">
        <span className="v2-calculator-policy-meta-label">Applicable</span>{' '}
        <span className="v2-status-badge v2-status-inactive">{profile.applicableYear}</span>
      </p>

      <dl className="v2-calculator-policy-list">
        <div>
          <dt>Physical calculator</dt>
          <dd>
            <PolicyStatus status={profile.physicalCalculator} />
          </dd>
        </div>
        <div>
          <dt>On-screen calculator</dt>
          <dd>
            <PolicyStatus status={profile.onScreenCalculator} />
          </dd>
        </div>
        <div>
          <dt>Function-level capabilities</dt>
          <dd>
            <PolicyStatus status={profile.functionSet} />
          </dd>
        </div>
      </dl>

      <p className="v2-calculator-policy-summary">{profile.summary}</p>

      {profile.caveats.length > 0 && (
        <ul className="v2-calculator-policy-caveats">
          {profile.caveats.map((caveat) => (
            <li key={caveat}>{caveat}</li>
          ))}
        </ul>
      )}

      {profile.keyboardNote && <p className="v2-calculator-policy-note">{profile.keyboardNote}</p>}

      <details className="v2-calculator-sources">
        <summary>Official sources</summary>
        <ol>
          {profile.sources.map((source) => (
            <li key={`${source.title}-${source.year}`}>
              <strong>{source.title}</strong>
              <span> ({source.year})</span>
              <p>{source.reference}</p>
              <code>{source.url}</code>
            </li>
          ))}
        </ol>
      </details>

      <p className="v2-calculator-policy-meta">Last verified: {profile.lastVerified}</p>
    </section>
  );
}
