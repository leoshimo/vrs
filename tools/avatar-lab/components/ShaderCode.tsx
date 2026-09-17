'use client';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from './ui/tooltip';

export const shaderTerms: Record<string, string> = {
  v: 'Output position relative to the center, measured in avatar radii.',
  px: 'Pixel coordinates in the drawing surface.',
  center: 'The avatar center in pixels.',
  radius: 'The avatar radius in pixels.',
  x: 'Horizontal coordinate; positive points right.',
  y: 'Vertical coordinate; positive points down.',
  sampleV:
    'Position used to read the field after displacement. The output pixel stays fixed.',
  displacement:
    'Offset between the output position and the field position it reads.',
  light: 'Direction of the coverage ramp.',
  dome: 'Height on the front half of a unit sphere. One at the center, zero at its edge.',
  distance:
    'Signed distance from the circle edge: negative inside, positive outside.',
  orbMask: 'Visibility at this point: 0 is transparent; 1 is fully visible.',
  tone: 'Ink coverage from zero to one, before printing turns it into marks.',
  phase:
    'An accumulated clock. Increasing phase advances the field along its path.',
  flow: 'Time-varying coverage added to the base, dome and directional gradient.',
  dot: 'GLSL built-in. Multiply corresponding vector components and add: ax·bx + ay·by.',
  length: 'GLSL built-in. Vector magnitude: √(x² + y²).',
  sqrt: 'GLSL built-in. Square root.',
  smoothstep:
    'GLSL built-in. Ease from 0 to 1 between two edges using t²(3−2t), with flat ends.',
  step: 'GLSL built-in. Return 0 below a threshold, otherwise 1.',
  mix: 'GLSL built-in. Blend a and b by t: a(1−t) + bt.',
  clamp: 'GLSL built-in. Keep a value between the lower and upper limits.',
  vec2: 'GLSL two-component vector, such as (x, y).',
  vec3: 'GLSL three-component vector, such as (x, y, z) or (r, g, b).',
  u_pitch:
    'Cell size in pixels, passed from the JavaScript controls into the shader.',
  u_phase: 'Current field phase, passed from the JavaScript clock.',
  u_fill:
    'Coverage remapping: 0 empties the avatar, .5 preserves it, 1 fills it.',
  u_volume: 'Weight of the hemisphere contribution to coverage.',
  u_lightStrength: 'Weight of the directional gradient.',
  u_lightAngle:
    'Gradient direction in radians inside the shader; the UI shows degrees.',
};
export function ShaderCode({ code }: { code: string }) {
  return (
    <TooltipProvider delay={250}>
      <code>
        {code.split(/(\b[A-Za-z_]\w*\b)/).map((token, i) =>
          shaderTerms[token] ? (
            <Tooltip key={i}>
              <TooltipTrigger
                render={
                  <button
                    className="shader-token"
                    type="button"
                    aria-label={token}
                  />
                }
              >
                {token}
              </TooltipTrigger>
              <TooltipContent>{shaderTerms[token]}</TooltipContent>
            </Tooltip>
          ) : (
            token
          ),
        )}
      </code>
    </TooltipProvider>
  );
}
