import type { VideoInfoReadOnlyResponse } from '../api/types';

export type VerdictLevel = 'real' | 'fake' | 'unknown';

export interface Verdict {
  level: VerdictLevel;
  /** Whether to show the "You Might've Been Scammed!" banner. */
  scammed: boolean;
  /** The big yes/no headline. */
  headline: string;
  /** The prominent model name / type shown above the metadata table. */
  modelLabel: string;
  /** Short supporting line under the headline. */
  subline: string;
}

const FRIENDLY_MODEL_NAMES: Record<string, string> = {
  veo: 'Google Veo',
  sora: 'OpenAI Sora',
  dreamina: 'Dreamina',
  kling: 'Kling',
};

/**
 * Seedance 2.0 model version as parsed by the backend from the model name
 * (`doubao-seedance-2-0` → `"2.0"`). A trailing numeric date suffix
 * (`doubao-seedance-2-0-260128` → `"2.0.260128"`) is still 2.0.
 */
const SEEDANCE_2_0_VERSION = /^2\.0(?:\.\d+)?$/;

/**
 * Decide the verdict from the API response.
 *
 * Real Seedance 2.0 requires a positive identity: a Seedance manifest whose
 * parsed version is 2.0 and that is not a Fast, Lite, or Mini variant. Older
 * Seedance versions, lesser variants, and manifests whose version can't be
 * read never get the affirmative verdict.
 */
export function classify(response: VideoInfoReadOnlyResponse): Verdict {
  const seedance = response.maybe_seedance;

  if (seedance) {
    const name = (seedance.model_name || '').toLowerCase();
    const version = seedance.maybe_model_version;

    if (!version) {
      return {
        level: 'unknown',
        scammed: true,
        headline: "We can't confirm this is Seedance 2.0",
        modelLabel: seedance.model_name,
        subline:
          'This video has Seedance provenance, but its model version could not be identified.',
      };
    }
    if (!SEEDANCE_2_0_VERSION.test(version)) {
      const lite = seedance.is_lite ? ' Lite' : '';
      return {
        level: 'fake',
        scammed: true,
        headline: `No! It's Seedance ${version}${lite} (not 2.0)`,
        modelLabel: seedance.model_name,
        subline: 'This is a different Seedance model — not Seedance 2.0.',
      };
    }
    if (seedance.is_fast || name.includes('-fast')) {
      return {
        level: 'fake',
        scammed: true,
        headline: "No! It's Seedance 2.0 Fast (not Full)",
        modelLabel: seedance.model_name,
        subline: 'This is the faster, lower-quality variant — not full Seedance 2.0.',
      };
    }
    if (name.includes('mini')) {
      return {
        level: 'fake',
        scammed: true,
        headline: "No! It's Seedance 2.0 Mini (not Full)",
        modelLabel: seedance.model_name,
        subline: 'This is the Mini variant — not full Seedance 2.0.',
      };
    }
    if (seedance.is_lite || name.includes('lite')) {
      return {
        level: 'fake',
        scammed: true,
        headline: "No! It's Seedance 2.0 Lite (not Full)",
        modelLabel: seedance.model_name,
        subline: 'This is the Lite variant — not full Seedance 2.0.',
      };
    }
    return {
      level: 'real',
      scammed: false,
      headline: "Yes! It's Real Seedance 2.0",
      modelLabel: seedance.model_name,
      subline: 'Verified Seedance provenance with a full-model signature.',
    };
  }

  const friendly = FRIENDLY_MODEL_NAMES[response.kind];
  if (friendly) {
    return {
      level: 'fake',
      scammed: true,
      headline: `No! It's not Seedance 2.0. It's ${friendly}`,
      modelLabel: friendly,
      subline: `This video carries ${friendly} provenance.`,
    };
  }

  return {
    level: 'unknown',
    scammed: true,
    headline: "We don't know what that is.",
    modelLabel: 'Unknown',
    subline:
      'No recognized AI-generation provenance was found — it may have been re-encoded, stripping the metadata.',
  };
}
