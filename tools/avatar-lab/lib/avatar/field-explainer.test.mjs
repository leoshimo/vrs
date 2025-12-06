import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  bayerThreshold,
  tutorialDensity,
  sampleDisplacement,
  paletteCoordinate,
  palettePigment,
  toneBand,
} from './field-explainer.ts';
test('Bayer explanation provides unique thresholds and correct coverage for every matrix', () => {
  for (const n of [2, 4, 8]) {
    const values = Array.from({ length: n * n }, (_, i) =>
      bayerThreshold(i % n, Math.floor(i / n), n),
    );
    assert.equal(new Set(values).size, n * n);
    assert.equal(values.filter((v) => 0.5 >= v).length, (n * n) / 2);
    assert.equal(values.filter((v) => 0 >= v).length, 0);
    assert.equal(values.filter((v) => 1 >= v).length, n * n);
  }
  assert.deepEqual(
    [
      bayerThreshold(0, 0, 2),
      bayerThreshold(1, 0, 2),
      bayerThreshold(0, 1, 2),
      bayerThreshold(1, 1, 2),
    ],
    [0.125, 0.625, 0.875, 0.375],
  );
});
test('isolating the dome removes directional asymmetry; angle reverses the gradient', () => {
  const sample = (x, angle, strength, stage) =>
    tutorialDensity(x, 0.1, 1, angle, strength, 0.5, stage);
  assert.equal(sample(0.5, 30, 1, 2), sample(-0.5, 30, 1, 2));
  assert.equal(sample(0.5, 0, 0, 3), sample(-0.5, 0, 0, 3));
  assert.ok(sample(0.5, 0, 1, 3) > sample(-0.5, 0, 1, 3));
  assert.ok(sample(0.5, 180, 1, 3) < sample(-0.5, 180, 1, 3));
});

test('coordinate operations retain identity at zero and translation is spatially uniform', () => {
  for (const op of ['translation', 'twist', 'shear', 'vertical']) {
    assert.ok(sampleDisplacement(0.3, -0.4, 1.2, 0, op).every((v) => v === 0));
  }
  assert.deepEqual(
    sampleDisplacement(0.8, -0.2, 1, 1, 'translation'),
    sampleDisplacement(-0.6, 0.3, 1, 1, 'translation'),
  );
  assert.notDeepEqual(
    sampleDisplacement(0.8, -0.2, 1, 1, 'shear'),
    sampleDisplacement(-0.6, 0.3, 1, 1, 'shear'),
  );
  assert.equal(sampleDisplacement(1.2, 0, 1, 1, 'vertical')[1], 0);
});
test('palette lookup changes over time independently of coverage; tone bands group density', () => {
  assert.equal(paletteCoordinate(0, 0, 0), 0.5);
  assert.notEqual(
    paletteCoordinate(0.2, 0.1, 0),
    paletteCoordinate(0.2, 0.1, 2),
  );
  assert.equal(toneBand(0.42, 5), toneBand(0.59, 5));
  assert.equal(toneBand(0.999, 5), 1);
  assert.equal(toneBand(0.6, 0), 0.6);
  const gray = palettePigment(0.3, false, 0, false);
  assert.equal(gray[0], gray[1]);
  assert.equal(gray[1], gray[2]);
});

test('surface wave joins at rest, scales linearly, and stays bounded across the sphere', async () => {
  const { surfaceSample } = await import('./field-explainer.ts');
  for (let x = -1.1; x <= 1.1; x += 0.11)
    for (let y = -1.1; y <= 1.1; y += 0.11) {
      for (const phase of [0, 4])
        assert.equal(Math.abs(surfaceSample(x, y, phase, 1).height), 0);
      for (let phase = 0.1; phase < 4; phase += 0.17) {
        const one = surfaceSample(x, y, phase, 1).height;
        const two = surfaceSample(x, y, phase, 2).height;
        assert.ok(Number.isFinite(one));
        assert.ok(Math.abs(one) <= 0.16);
        assert.ok(Math.abs(two - 2 * one) < 1e-10);
      }
    }
});
test('light-kick demonstration is the critically damped one-impulse response', async () => {
  const { impulseSample } = await import('./field-explainer.ts');
  const peak = impulseSample(1 / 13, 1);
  assert.ok(Math.abs(peak.velocity) < 1e-10);
  assert.ok(peak.value > impulseSample(0.5, 1).value);
  assert.equal(impulseSample(0, 1).value, 0);
  assert.equal(impulseSample(0, 1).velocity, 16);
  assert.ok(impulseSample(1, 1).value < 0.001);
});
