/**
 * Where this process is running.
 *
 * The same four names, aliases and rules as `Environment` in
 * `crates/telemetry`, so the web app and the services never disagree about
 * which environment they are in.
 */
export const ENVIRONMENTS = ['local', 'ci', 'staging', 'production'] as const;

export type Environment = (typeof ENVIRONMENTS)[number];

/** Names we accept for an environment, beyond the four canonical ones. */
const ALIASES: Readonly<Record<string, Environment>> = {
  dev: 'local',
  development: 'local',
  test: 'ci',
  prod: 'production'
};

/** What we assume when nothing says otherwise. */
export const DEFAULT_ENVIRONMENT: Environment = 'local';

/**
 * Read an environment name, as `CRONCAVE_ENV` holds it.
 *
 * Unset or blank means {@link DEFAULT_ENVIRONMENT}. An unrecognised value
 * throws rather than being guessed at: a typo in a deployment's configuration
 * should stop the app, not silently make production look local.
 */
export function parseEnvironment(value: string | undefined | null): Environment {
  const name = value?.trim().toLowerCase();
  if (!name) {
    return DEFAULT_ENVIRONMENT;
  }

  if ((ENVIRONMENTS as readonly string[]).includes(name)) {
    return name as Environment;
  }

  const alias = ALIASES[name];
  if (alias) {
    return alias;
  }

  throw new Error(
    `unknown CRONCAVE_ENV value ${JSON.stringify(value)}: expected ${ENVIRONMENTS.join(', ')}`
  );
}

/** Whether this environment serves real users. */
export function isDeployed(environment: Environment): boolean {
  return environment === 'staging' || environment === 'production';
}
