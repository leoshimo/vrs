import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
register('./test-resolve.mjs', import.meta.url);
const { resolveExpression, editExpression, isModified } =
  await import('./expression-config.ts');
const { defaultSignal } = await import('./signal-field.ts');
const { expressions } = await import('./avatar-expressions.ts');
const { expressionEffect } = await import('./avatar-effects.ts');
const { appearanceMask } = await import('./appearance-mask.ts');
test('expression overrides cannot overwrite base appearance or unrelated actions', () => {
  const base = { ...defaultSignal, color: 'opal', fill: 0.35, pitch: 2 };
  const e = expressions.find((e) => e.id === 'drift');
  const override = editExpression(e, undefined, {
    settings: { loadingRate: 1.6, color: 'ember', fill: 1, typingEnergy: 3 },
  });
  const { config } = resolveExpression(base, e, override);
  assert.equal(config.loadingRate, 1.6);
  assert.equal(config.color, 'opal');
  assert.equal(config.fill, 0.35);
  assert.equal(config.pitch, 2);
  assert.equal(config.typingEnergy, base.typingEnergy);
  assert.deepEqual(base, {
    ...defaultSignal,
    color: 'opal',
    fill: 0.35,
    pitch: 2,
  });
});
test('resetting an override restores the preset and changing expression does not leak settings', () => {
  const wave = expressions.find((e) => e.id === 'surface-submit'),
    drift = expressions.find((e) => e.id === 'drift');
  const override = editExpression(wave, undefined, {
    settings: { surfaceWidth: 0.6, surfaceOriginX: 0.7 },
  });
  assert.equal(isModified(override), true);
  assert.equal(
    resolveExpression(defaultSignal, wave, override).config.surfaceWidth,
    0.6,
  );
  assert.equal(
    resolveExpression(defaultSignal, wave).config.surfaceWidth,
    0.32,
  );
  assert.equal(
    resolveExpression(defaultSignal, drift).config.surfaceOriginX,
    -0.3,
  );
  const returned = editExpression(wave, override, {
    settings: { surfaceWidth: 0.32, surfaceOriginX: -0.3 },
  });
  assert.equal(isModified(returned), false);
});
test('appearance duration and target belong to each expression', () => {
  const fill = expressions.find((e) => e.id === 'fill-wake'),
    print = expressions.find((e) => e.id === 'print-wake');
  const override = editExpression(fill, undefined, {
    duration: 2.8,
    settings: { appearanceTarget: 'bar' },
  });
  assert.equal(resolveExpression(defaultSignal, fill, override).duration, 2.8);
  assert.equal(
    resolveExpression(defaultSignal, fill, override).config.appearanceTarget,
    'bar',
  );
  assert.equal(resolveExpression(defaultSignal, print).duration, 1.4);
  assert.equal(
    resolveExpression(defaultSignal, print).config.appearanceTarget,
    'avatar',
  );
  assert.equal(
    isModified(
      editExpression(fill, override, {
        duration: 1.4,
        settings: { appearanceTarget: 'avatar' },
      }),
    ),
    false,
  );
});
test('surface expressions share a formula and drift and eddies are distinct effects', () => {
  assert.equal(
    expressionEffect(expressions.find((e) => e.id === 'surface-submit')),
    'surface',
  );
  assert.equal(
    expressionEffect(expressions.find((e) => e.id === 'surface-working')),
    'surface',
  );
  assert.notEqual(
    expressionEffect(expressions.find((e) => e.id === 'drift')),
    expressionEffect(expressions.find((e) => e.id === 'eddies')),
  );
});
test('whole bar appearance finishes without a residual mask', () => {
  for (const mode of ['fill', 'print']) {
    assert.equal(appearanceMask(1, mode), 'none');
    assert.equal(appearanceMask(3, mode), 'none');
    assert.notEqual(appearanceMask(0, mode), appearanceMask(0.5, mode));
    assert.equal(appearanceMask(0.501, mode), appearanceMask(0.502, mode));
  }
});
