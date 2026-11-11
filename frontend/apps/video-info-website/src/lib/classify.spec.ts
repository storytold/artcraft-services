import { describe, expect, it } from 'vitest';

import type { SeedanceVideoInfo, VideoInfoReadOnlyResponse } from '../api/types';
import { classify } from './classify';

function seedance(
  model_name: string,
  maybe_model_version: string | null,
  flags: { is_fast?: boolean; is_lite?: boolean } = {},
): VideoInfoReadOnlyResponse {
  const info = {
    model_name,
    maybe_model_version,
    is_fast: flags.is_fast ?? false,
    is_lite: flags.is_lite ?? false,
  } as SeedanceVideoInfo;
  return {
    success: true,
    kind: 'seedance',
    maybe_encoder: null,
    maybe_seedance: info,
    maybe_veo: null,
    maybe_sora: null,
    maybe_dreamina: null,
    maybe_kling: null,
  };
}

describe('classify', () => {
  it('accepts full Seedance 2.0', () => {
    expect(classify(seedance('doubao-seedance-2-0', '2.0')).level).toBe('real');
    expect(classify(seedance('dreamina-seedance-2-0', '2.0')).level).toBe('real');
    expect(classify(seedance('doubao-seedance-2-0-260128', '2.0.260128')).level).toBe('real');
  });

  it('rejects older and Lite Seedance models', () => {
    const verdict = classify(seedance('doubao-seedance-1-0-lite', '1.0', { is_lite: true }));
    expect(verdict.level).toBe('fake');
    expect(verdict.scammed).toBe(true);
    expect(classify(seedance('doubao-seedance-1-0-pro', '1.0')).level).toBe('fake');
    expect(classify(seedance('doubao-seedance-2-0-lite', '2.0', { is_lite: true })).level).toBe('fake');
  });

  it('rejects Fast and Mini 2.0 variants', () => {
    expect(classify(seedance('doubao-seedance-2-0-fast', '2.0', { is_fast: true })).level).toBe('fake');
    expect(classify(seedance('doubao-seedance-2-0-mini', '2.0')).level).toBe('fake');
  });

  it('does not affirm a Seedance model whose version is unknown', () => {
    const verdict = classify(seedance('seedance', null));
    expect(verdict.level).toBe('unknown');
    expect(verdict.scammed).toBe(true);
  });
});
