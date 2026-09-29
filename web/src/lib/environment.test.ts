import { describe, expect, it } from 'vitest';
import { DEFAULT_ENVIRONMENT, isDeployed, parseEnvironment } from './environment';

describe('parseEnvironment', () => {
  it('accepts the four names', () => {
    expect(parseEnvironment('local')).toBe('local');
    expect(parseEnvironment('ci')).toBe('ci');
    expect(parseEnvironment('staging')).toBe('staging');
    expect(parseEnvironment('production')).toBe('production');
  });

  it('accepts the same aliases the Rust side does', () => {
    expect(parseEnvironment('DEV')).toBe('local');
    expect(parseEnvironment(' prod ')).toBe('production');
    expect(parseEnvironment('test')).toBe('ci');
  });

  it('falls back to the default when unset or blank', () => {
    expect(parseEnvironment(undefined)).toBe(DEFAULT_ENVIRONMENT);
    expect(parseEnvironment('')).toBe(DEFAULT_ENVIRONMENT);
    expect(parseEnvironment('   ')).toBe(DEFAULT_ENVIRONMENT);
  });

  it('refuses a name it does not know', () => {
    expect(() => parseEnvironment('eu-west')).toThrow(/unknown CRONCAVE_ENV/);
  });
});

describe('isDeployed', () => {
  it('is true only where real users are', () => {
    expect(isDeployed('production')).toBe(true);
    expect(isDeployed('staging')).toBe(true);
    expect(isDeployed('local')).toBe(false);
    expect(isDeployed('ci')).toBe(false);
  });
});
