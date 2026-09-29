import { describe, expect, it } from 'vitest';
import { DATA_COLLECTION } from './sentry';

describe('DATA_COLLECTION', () => {
  const fields = Object.entries(DATA_COLLECTION);

  it('covers every field Sentry defaults to collecting', () => {
    expect(fields.length).toBeGreaterThan(0);
    expect(Object.keys(DATA_COLLECTION)).toContain('cookies');
    expect(Object.keys(DATA_COLLECTION)).toContain('httpHeaders');
  });

  // Sentry's defaults are permissive and new releases add fields. This fails
  // if anything here is ever switched on, deliberately or by a merge.
  it('permits nothing', () => {
    for (const [field, value] of fields) {
      const collectsNothing = value === false || (Array.isArray(value) && value.length === 0);
      expect(collectsNothing, `${field} must collect nothing, got ${JSON.stringify(value)}`).toBe(
        true
      );
    }
  });
});
