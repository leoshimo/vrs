// Resolve TypeScript modules' extensionless relative imports for Node's strip-types runner.
export async function resolve(specifier, context, next) {
  try {
    return await next(specifier, context);
  } catch (error) {
    if (
      error.code === 'ERR_MODULE_NOT_FOUND' &&
      specifier.startsWith('.') &&
      !/\.[a-z]+$/i.test(specifier)
    ) {
      return next(`${specifier}.ts`, context);
    }
    throw error;
  }
}
