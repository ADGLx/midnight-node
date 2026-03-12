import { describe, it, expect } from 'vitest';
import {
  getCommandPackage,
  getSupportedVersions,
  getDefaultVersion,
} from '../src/version-registry.js';

describe('version-registry', () => {
  it('returns the correct package for 0.29.0', () => {
    expect(getCommandPackage('0.29.0')).toBe('@midnight-ntwrk/compact-js-command');
  });

  it('returns the correct package for 0.30.0-rc.0', () => {
    expect(getCommandPackage('0.30.0-rc.0')).toBe('@midnight-ntwrk/compact-js-command-v2-5-0');
  });

  it('throws for an unknown version', () => {
    expect(() => getCommandPackage('0.99.0')).toThrow(
      'Unsupported compact version: 0.99.0'
    );
  });

  it('includes supported versions in the error message', () => {
    expect(() => getCommandPackage('0.99.0')).toThrow('0.29.0');
    expect(() => getCommandPackage('0.99.0')).toThrow('0.30.0-rc.0');
  });

  it('lists supported versions', () => {
    expect(getSupportedVersions()).toEqual(['0.29.0', '0.30.0-rc.0']);
  });

  it('returns 0.30.0-rc.0 as the default version', () => {
    expect(getDefaultVersion()).toBe('0.30.0-rc.0');
  });

  it('returns a default version that is supported', () => {
    const defaultVersion = getDefaultVersion();
    expect(getSupportedVersions()).toContain(defaultVersion);
  });
});
