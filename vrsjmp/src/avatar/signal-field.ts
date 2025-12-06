export type SignalShape = "pearl" | "square" | "charged" | "duet" | "ribbon" | "gentle";
export type SignalColor =
  | "mono"
  | "aurora"
  | "ember"
  | "lagoon"
  | "oxide"
  | "risograph"
  | "field"
  | "opal"
  | "spectrum"
  | "slate"
  | "moss"
  | "plum"
  | "ochre"
  | "mist"
  | "dusk";
export type SignalPlacement = "leading" | "trailing" | "floating" | "above";
export type EffectTargets = {
  indicator: boolean;
  selection: boolean;
  input: boolean;
  container: boolean;
  placeholder: boolean;
};
export type SignalConfig = {
  shape: SignalShape;
  placement: SignalPlacement;
  breathe: boolean;
  selection: "fill" | "shadow";
  texture: "bayer" | "halftone";
  effects: EffectTargets;
  edge: "bottom" | "leading" | "wave" | "all";
  amount: number;
  pitch: number;
  expression: number;
  language?: "porcelain" | "orbital" | "seal";
  color?: SignalColor;
  colorTiming?: "always" | "reactive";
  loadingMotion?:
    | "orbit"
    | "alternating"
    | "bands"
    | "strong"
    | "right-left"
    | "drift"
    | "quicker"
    | "agitated"
    | "left-right"
    | "top-bottom"
    | "bottom-top"
    | "random"
    | "eddy"
    | "radial"
    | "surface";
  typingMotion?:
    | "dots"
    | "both"
    | "cross-dots"
    | "cross-both"
    | "pressure"
    | "light"
    | "agitation"
    | "nudge";
  interior?: "light" | "spheres";
  idleAmount?: number;
  typingEnergy?: number;
  dispatchEnergy?: number;
  settle?: number;
  coupling?: number;
  boundary?: "none" | "inner" | "outer";
  shapeNoise?: number;
  toneSteps?: number;
  colorStrength?: number;
  rippleAccent?: boolean;
  sparkCount?: number;
  inspectPoke?: [number, number];
  inspectPhase?: number; // Walkthrough scrubber; undefined uses the motion clock.
  loopRipple?: Partial<SignalConfig>;
  surfaceWave?: number; // Experimental radial surface displacement.
  surfacePhase?: number;
  surfaceAmplitude?: number;
  surfaceDuration?: number;
  surfaceWidth?: number;
  surfaceWavelength?: number;
  surfaceDamping?: number;
  surfaceOriginX?: number;
  surfaceOriginY?: number;
  surfaceSilhouette?: number;
  surfaceDisplacement?: number;
  surfaceShading?: number;
  appearanceTarget?: "avatar" | "bar";
  effectMix?: {
    drift: number;
    eddies: number;
    agitation: number;
    wave?: number;
  };
  lavaMix?: number;
  effectLayerSettings?: Partial<
    Record<"drift" | "eddies" | "agitation" | "wave", Partial<SignalConfig>>
  >;
  debugStage?: number; // Tutorial only; 0 means complete field.

  saturation?: number;
  activitySaturation?: number;
  gravity?: number;
  primitiveDrift?: number;
  primitiveTwist?: number;
  primitiveNoise?: number;
  primitivePigment?: number;
  volume?: number;
  lightStrength?: number;
  lightAngle?: number;
  loadingStrength?: number;
  loadingRate?: number;
  driftAngle?: number;
  waveOffset?: number;
  appearanceMode?: "print" | "fill";
  uiAccentPalette?: SignalColor;
  uiFont?: "system" | "humanist" | "grotesk" | "mono";
  uiSurface?: "porcelain" | "solar" | "mineral";
  placeholderMode?: "plain" | "breathe" | "sheen" | "arrive" | "print";
  selectionMotion?: "glide" | "crossfade" | "instant";
  selectionTint?: number;
  selectionEffect?: "none" | "settle" | "wick" | "pool" | "inset" | "seam" | "trail" | "print";
  selectionInk?: number;
  selectionGrain?: number;
  selectionTexture?: "bayer" | "halftone";
  selectionDuration?: number;
  fill?: number;
  contrast?: number;
  matrix?: 2 | 4 | 8;
  dispatchTarget?: "avatar" | "selection";
  avatarDispatch?: "gather" | "bloom" | "sparks" | "ripple" | "surface";
  selectionRelease?: "contour" | "sweep" | "press";
  inputTreatment?: "top" | "bottom" | "shadow" | "flat-top" | "flat-bottom";
  edgeSignal?: "off" | "leading" | "bottom";
  edgeMotion?: "wave" | "traveler" | "alternating";
};
export const defaultSignal: SignalConfig = {
  shape: "pearl",
  placement: "leading",
  breathe: true,
  selection: "fill",
  texture: "bayer",
  effects: {
    indicator: true,
    selection: false,
    input: false,
    container: false,
    placeholder: false,
  },
  edge: "wave",
  amount: 0.22,
  pitch: 1.15,
  expression: 1.25,
  color: "mono",
  colorTiming: "reactive",
  loadingMotion: "alternating",
  edgeSignal: "off",
  edgeMotion: "wave",
  typingMotion: "light",
  interior: "light",
  idleAmount: 1.2,
  typingEnergy: 1.15,
  dispatchEnergy: 1.2,
  settle: 1.5,
  coupling: 1,
  boundary: "none",
  shapeNoise: 0,
  toneSteps: 0,
  colorStrength: 0.8,
  rippleAccent: false,
  sparkCount: 10,
  surfaceAmplitude: 0.8,
  surfaceDuration: 3,
  surfaceWidth: 0.32,
  surfaceWavelength: Math.PI / 6,
  surfaceDamping: 0,
  surfaceOriginX: -0.3,
  surfaceOriginY: -0.45,
  surfaceSilhouette: 1,
  surfaceDisplacement: 1,
  surfaceShading: 1,
  appearanceTarget: "avatar",
  fill: 0.5,
  contrast: 1,
  matrix: 8,
  saturation: 0.45,
  activitySaturation: 0.65,
  gravity: 0,
  volume: 1,
  lightStrength: 1,
  lightAngle: 0,
  loadingStrength: 1,
  loadingRate: 1,
  driftAngle: 0,
  waveOffset: 0,
  appearanceMode: "fill",
  uiFont: "system",
  uiSurface: "porcelain",
  placeholderMode: "plain",
  selectionMotion: "crossfade",
  selectionTint: 0.1,
  dispatchTarget: "avatar",
  avatarDispatch: "gather",
  selectionRelease: "contour",
  inputTreatment: "bottom",
};

// Shared by the renderer and the Anatomy code view. Phase runs from 0 to 4.
export const surfaceWaveGLSL = `float surfaceHeightAt(vec2 v, float phase, float amplitude, vec2 originPoint, vec3 profile) {
  if (phase <= 0. || phase >= 3.8 || amplitude <= 0.) return 0.;
  vec3 point = normalize(vec3(v, sqrt(max(0., 1. - dot(v, v)))));
  vec3 origin = normalize(vec3(originPoint, .84));
  float angle = acos(clamp(dot(point, origin), -1., 1.));
  float front = phase * .82;
  float distance = angle - front;
  float envelope = exp(-pow(distance / max(.01, profile.x), 2.))
    * smoothstep(0., .22, phase)
    * (1. - smoothstep(2.8, 3.8, phase));
  return sin(distance * 6.28318530718 / max(.01, profile.y))
    * envelope * exp(-profile.z * front) * amplitude * .16;
}
float surfaceHeight(vec2 v, float phase, float amplitude) { return surfaceHeightAt(v,phase,amplitude,u_surfaceOrigin,u_surfaceProfile); }`;

export const displacementEffectsGLSL = `vec2 driftOffset(vec2 v, float transport) {
  return vec2(sin(v.y*2.4+transport), cos(v.x*2.-transport))*.12;
}
vec2 eddyOffset(vec2 v, float phase) {
  vec2 center=vec2(.24*sin(phase*1.1), .22*cos(phase*.83));
  vec2 q=v-center;
  return vec2(-q.y,q.x)*exp(-dot(q,q)*1.7)*.8*sin(phase*2.);
}
vec2 agitationField(vec2 v, float phase) {
  return vec2(sin(v.y*4.+phase*2.7)+.35*sin(v.y*7.-phase*1.9),
    cos(v.x*3.4-phase*2.1));
}`;

// Original GLSL. Independent region masks share one lattice and continuous motion.
export const signalFragment = `#version 300 es
precision highp float;
uniform vec2 u_resolution;
uniform float u_pixelRatio,u_time,u_breathe,u_motion,u_shape,u_texture,u_dark,u_selectAt,u_appearance,u_selectionStyle;
uniform float u_phase,u_attack,u_loading,u_dispatch,u_tap,u_burst,u_echo,u_edge,u_amount,u_pitch,u_expression,u_enableOrb,u_language;
uniform float u_split,u_selectDuration;
uniform float u_color,u_colorTiming,u_loadingMotion,u_edgeSignal,u_edgeMotion,u_releaseAge,u_releasePhase;
uniform float u_priorReleaseAge,u_priorReleasePhase;
uniform float u_typingStyle,u_inputStyle,u_interior,u_idleAmount,u_fill,u_wake,u_seed,u_transport,u_dispatchTarget,u_releaseStyle,u_releaseRadius,u_contrast,u_matrix,u_releaseMode;
uniform float u_coupling,u_boundary,u_shapeNoise,u_toneSteps,u_colorStrength,u_rippleAccent,u_sparkCount,u_momentum,u_debugStage;
uniform float u_saturation,u_activitySaturation,u_gravity,u_volume,u_lightStrength,u_lightAngle,u_driftAngle,u_waveOffset,u_fillWake,u_selectionMotion,u_selectionTint;
uniform vec3 u_uiAccent,u_selectionInkColor;
uniform vec4 u_primitives;
uniform vec2 u_loopRipple,u_loopOrigin;
uniform vec3 u_loopProfile,u_loopInfluence;
uniform vec2 u_surfaceWave,u_surfaceExpression,u_surfaceOrigin;
uniform vec3 u_surfaceProfile,u_surfaceInfluence,u_effectMix;
uniform float u_lavaMix,u_mixing,u_mixCustom,u_mixWaveMode,u_mixWaveWeight,u_mixWaveAccent,u_mixDriftAngle;
uniform vec4 u_mixDrift,u_mixEddies,u_mixAgitation,u_mixWave;
uniform float u_rowEffect,u_rowAmount,u_rowPitch,u_rowTexture;
uniform vec4 u_releaseRow;
uniform vec2 u_poke,u_poke2;
uniform vec4 u_radii;
uniform vec3 u_paperColor,u_inkColor,u_rowColor;
uniform vec4 u_targets,u_signal,u_panel,u_query,u_island,u_selection,u_previous,u_clip;
out vec4 fragColor;
${surfaceWaveGLSL}
${displacementEffectsGLSL}
float box(vec2 p,vec4 r,float radius){vec2 q=abs(p-r.xy-r.zw*.5)-r.zw*.5+radius;return length(max(q,0.))+min(max(q.x,q.y),0.)-radius;}
float maskBox(vec2 p,vec4 r,float radius){return (1.-smoothstep(-.6,.6,box(p,r,radius)))*step(.5,r.w)*step(.5,r.z);}
float b2(vec2 p){vec2 q=mod(floor(p),2.);return 2.*q.x+3.*q.y-4.*q.x*q.y;}
float rank8(vec2 p){return (16.*b2(p)+4.*b2(floor(p/2.))+b2(floor(p/4.))+.5)/64.;}
float printed(float density,vec2 px){
  density=clamp(density,0.,1.);
  if(u_texture<.5){
    vec2 cell=floor(px/u_pitch);
    float threshold=u_matrix<3.?(b2(cell)+.5)/4.:u_matrix<5.?(4.*b2(cell)+b2(floor(cell/2.))+.5)/16.:rank8(cell);
    return step(threshold,density)*step(.001,density);
  }
  float spacing=u_pitch*2.;vec2 cell=fract(px/spacing)-.5;float r=sqrt(density)*.7;
  return (1.-smoothstep(r-.05,r+.05,length(cell)))*step(.012,density);
}
float rowPrint(float density,vec2 px){
  density=clamp(density,0.,1.);
  float pitch=max(.6,u_rowPitch);
  if(u_rowTexture<.5)return step(rank8(floor(px/pitch)),density)*step(.001,density);
  vec2 cell=fract(px/(pitch*2.))-.5;
  float r=sqrt(density)*.70;
  return (1.-smoothstep(r-.035,r+.035,length(cell)))*step(.001,density);
}
float smoothUnion(float a,float b,float k){float h=max(k-abs(a-b),0.)/k;return min(a,b)-h*h*k*.25;}
float blobDistance(vec2 v,float phase){
  float a=atan(v.y,v.x);
  return length(v)-1.-.085*sin(a*2.+phase)-.04*sin(a*3.-phase*.7);
}
vec2 twinOffset(float phase){return vec2(.29+.05*sin(phase),.17*cos(phase*.8));}
float signalDistance(vec2 v,float phase){
  if(u_shape>4.5){
    float a=atan(v.y,v.x);
    return length(v)-1.-.026*sin(a*2.+phase*1.3)-.012*sin(a*3.-phase*.9);
  }
  if(u_shape>3.5){
    float y=.13*sin(v.x*3.2+phase*2.)+.065*sin(v.x*6.-phase*1.4);
    vec2 q=vec2(v.x/1.19,(v.y-y)/(.40+.055*cos(phase)));
    return (length(q)-1.)*.65;
  }
  if(u_shape>2.5){
    vec2 offset=twinOffset(phase);
    return smoothUnion(blobDistance((v-offset)/.74,phase)*.74,blobDistance((v+offset)/.72,-phase+1.7)*.72,.24);
  }
  if(u_shape>1.5)return blobDistance(v,phase*1.8);
  if(u_shape>.5){vec2 q=abs(v)-.62;return length(max(q,0.))+min(max(q.x,q.y),0.)-.24;}
  return length(v)-1.;
}
float activityLevel(){return clamp(u_tap*1.6+u_loading*.8+u_burst*.9*(1.-step(.5,u_dispatchTarget))+u_wake*.55,0.,1.);}
float colorEnergy(){return u_colorTiming<.5?1.:activityLevel();}
vec3 pigment(vec2 p,float phase){
  float t=.5+.5*sin(p.x*1.8+p.y*.9+phase*.7);
  float s=.5+.5*cos(p.y*2.-p.x*.8-phase*.5);
  vec3 a=u_color<1.5?vec3(.10,.68,.72):vec3(.95,.29,.15);
  vec3 b=u_color<1.5?vec3(.43,.36,.94):vec3(.96,.63,.22);
  vec3 c=u_color<1.5?vec3(.95,.40,.68):vec3(.71,.23,.54);
  vec3 col=mix(mix(a,b,t),c,s*.48);
  if(u_color>2.5&&u_color<3.5)col=mix(vec3(.08,.42,.62),vec3(.85,.91,.68),smoothstep(.38,.62,t));
  if(u_color>3.5&&u_color<4.5)col=mix(vec3(.80,.27,.17),vec3(.12,.65,.59),smoothstep(.38,.62,t));
  if(u_color>4.5){
    float band=floor(clamp(t*.7+s*.3,0.,.999)*3.);
    col=band<1.?vec3(.08,.68,.79):band<2.?vec3(.91,.25,.52):vec3(.96,.81,.29);
  }
  if(u_color>5.5){
    // Original gradients, inspired by color-coded instruments, not sampled brand colors.
    vec3 x=vec3(.18,.52,.89), y=vec3(.52,.77,.60), z=vec3(.97,.59,.27);
    if(u_color>6.5&&u_color<7.5){x=vec3(.52,.68,.90);y=vec3(.76,.61,.83);z=vec3(.93,.80,.61);}
    if(u_color>7.5){x=vec3(.27,.37,.87);y=vec3(.76,.41,.61);z=vec3(.94,.70,.40);}
    col=t<.5?mix(x,y,smoothstep(0.,.5,t)):mix(y,z,smoothstep(.5,1.,t));
  }
  if(u_color>8.5&&u_color<12.5){
    col=u_color<9.5?vec3(.39,.52,.64):u_color<10.5?vec3(.43,.57,.45):u_color<11.5?vec3(.62,.46,.59):vec3(.68,.55,.31);
    col*=.82+t*.25;
  }
  if(u_color>12.5)col=u_color<13.5?mix(vec3(.53,.67,.70),vec3(.77,.73,.66),t):mix(vec3(.54,.51,.68),vec3(.76,.63,.56),t);
  float luminance=dot(col,vec3(.2126,.7152,.0722));
  col=mix(vec3(luminance),col,mix(u_saturation,u_activitySaturation,activityLevel()));
  return mix(col*.7,col+.12,u_dark);
}
vec3 signalInk(vec2 p,float phase){return mix(u_inkColor,pigment(p,phase),step(.5,u_color)*colorEnergy()*u_colorStrength);}
float liveEdge(vec2 px,vec4 rect){
  bool leading=u_edgeSignal<1.5;
  float along=leading?(px.y-rect.y)/max(1.,rect.w):(px.x-rect.x)/max(1.,rect.z);
  float depth=leading?px.x-rect.x:rect.y+rect.w-px.y;
  float clock=u_phase*4.;
  float amplitude=.10+u_loading*.55+u_tap*.4;
  float field;
  if(u_edgeMotion<.5){
    float w=sin(along*24.-clock*3.)*.55+sin(along*43.+clock*2.)*.25;
    float envelope=pow(sin(clamp(along,0.,1.)*3.14159),.65);
    field=exp(-pow((depth-(5.+(5.+w*8.)*envelope*amplitude))/5.5,2.));
  }else if(u_edgeMotion<1.5){
    float head=fract(clock*.45);
    float gap=abs(along-head);gap=min(gap,1.-gap);
    field=exp(-pow(gap/.14,2.))*exp(-pow((depth-5.)/5.,2.));
  }else{
    float cycle=clock*.65;
    float travel=fract(cycle);
    float origin=mod(floor(cycle),2.)<1.?0.:1.;
    field=exp(-pow((abs(along-origin)-travel)/.18,2.))*pow(sin(travel*3.14159),.65)*exp(-pow((depth-5.)/6.,2.));
  }
  return field*amplitude*step(0.,depth)*step(along,1.)*step(0.,along);
}
float edgeTone(vec2 px,vec4 rect){
  float edge=box(px,rect,min(27.,rect.w*.45));
  float inside=maskBox(px,rect,min(27.,rect.w*.45));
  float along=px.x-rect.x;
  float rough=sin(floor(along/3.)*1.71)*1.4;
  float wave=(sin(along*.029+u_phase*1.3)+.45*sin(along*.071-u_phase))*(2.+u_attack*5.+u_loading*2.);
  float depth=rect.y+rect.w-px.y;
  if(u_edge<.5)return exp(-max(0.,depth-2.-rough)/5.)*inside;
  if(u_edge<1.5)return exp(-max(0.,px.x-rect.x-2.-rough)/5.)*inside;
  if(u_edge<2.5)return exp(-max(0.,depth-8.-wave*1.6-rough)/9.)*inside;
  return exp(min(0.,edge)/6.)*inside;
}
void main(){
  vec2 px=vec2(gl_FragCoord.x,u_resolution.y-gl_FragCoord.y)/u_pixelRatio;
  vec3 paper=u_paperColor;
  vec3 ink=u_inkColor;
  // Loading follows a flatter, tilted orbit rather than just speeding up idle.
  float lightAngle=u_phase-.7+.24*sin(u_phase*1.7)+u_lightAngle;
  vec2 light=vec2(cos(lightAngle),sin(lightAngle)*(1.-.45*u_loading));
  float tilt=u_loading*.32;
  light=mat2(cos(tilt),-sin(tilt),sin(tilt),cos(tilt))*light;
  if(u_mixCustom>.5)light=mix(vec2(cos(-.7+u_lightAngle),sin(-.7+u_lightAngle)),light,u_lavaMix);
  // The original single-light interior responds without warping its silhouette.
  if(u_typingStyle>4.5&&u_interior<.5)light+=u_poke*.65*u_expression;
  float bodyRadius=min(u_radii.x,min(u_panel.z,u_panel.w)*.5);
  float body=maskBox(px,u_panel,bodyRadius);
  float query=maskBox(px,u_query,min(u_radii.y,u_query.w*.5));
  float island=maskBox(px,u_island,u_island.w*.5);
  float base=max(body,max(query*u_split,island));
  vec3 color=paper*base;float alpha=base;
  float rawMove=u_selectDuration>0.?clamp((u_time-u_selectAt)/u_selectDuration,0.,1.):1.;
  float move=u_selectDuration>0.?clamp((u_time-u_selectAt)/u_selectDuration,0.,1.):1.;move=mix(1.,1.-pow(1.-move,3.),u_motion);
  vec4 row=u_selectionMotion>.5?u_selection:mix(u_previous,u_selection,move);
  float rowRadius=min(u_radii.z,row.w*.5);
  float rowD=box(px,row,rowRadius);
  float clip=step(u_clip.x,px.x)*step(u_clip.y,px.y)*step(px.x,u_clip.x+u_clip.z)*step(px.y,u_clip.y+u_clip.w);
  float rowMask=maskBox(px,row,rowRadius)*clip*body;
  if(u_selectionMotion>.5&&u_selectionMotion<1.5){
    float oldMask=maskBox(px,u_previous,rowRadius)*clip*body;
    rowMask=mix(oldMask,rowMask,move);
  }
  float wave=.5+.5*sin(dot(px-u_signal.xy,light)*.025-u_phase*.4);
  float density=u_selectionStyle>.5?exp(min(0.,rowD)/6.)*.5:.2+.23*wave;
  vec3 rowBase=mix(u_rowColor,u_uiAccent,u_selectionTint);
  vec3 rowColor=mix(rowBase,vec3(.22,.29,.24),printed(density,px)*u_targets.y*.25);
  // Row expressions share the field's print language, but have independent controls.
  // They never displace text or carry an old-row impulse into a new page.
  float currentRow=maskBox(px,u_selection,min(u_radii.z,u_selection.w*.5))*clip*body;
  vec2 rp=px-u_selection.xy;
  vec2 uv=rp/max(vec2(1.),u_selection.zw);
  float pulse=sin(rawMove*3.14159);
  float inkPattern=0.;
  vec3 selectionInk=mix(rowBase,u_selectionInkColor,.26);
  if(u_rowEffect>.5&&u_rowEffect<1.5){
    // Ink settles from discrete islands into a quiet solid field.
    float hills=.5+.5*sin(uv.x*10.+uv.y*3.)*cos(uv.y*5.-uv.x*2.);
    float wet=clamp(rawMove*1.65-hills*.40,0.,1.);
    float marks=rowPrint(wet,px);
    rowMask=max(rowMask,currentRow*(.66+.34*marks));
    inkPattern=rowPrint(pulse*(.16+.32*hills),px)*pulse;
  }else if(u_rowEffect>1.5&&u_rowEffect<2.5){
    // A ragged capillary front advances across the selected surface.
    float grainLine=sin(floor(rp.y/max(.6,u_rowPitch))*.91)*.017+sin(uv.y*13.)*.022;
    float front=-.08+rawMove*1.2+grainLine;
    float band=exp(-pow((uv.x-front)/.055,2.));
    inkPattern=rowPrint(band*.7,px)*pulse;
    rowMask=max(rowMask,currentRow*.78);
  }else if(u_rowEffect>2.5&&u_rowEffect<3.5){
    // A local pressure field expands elliptically; the dots bend around it.
    vec2 q=(uv-vec2(.23,.5))*vec2(1.,.32);
    float r=length(q),front=rawMove*1.05;
    float ring=exp(-pow((r-front)/.13,2.))*pulse;
    vec2 bend=q/max(.03,r)*ring*8.;
    inkPattern=rowPrint(ring*(.45+.14*sin(uv.x*12.)),px-bend);
    rowMask=max(rowMask,currentRow*.78);
  }else if(u_rowEffect>3.5&&u_rowEffect<4.5){
    // Printed inner shadow settles into the same rounded rectangle.
    float depth=-box(px,u_selection,min(u_radii.z,u_selection.w*.5));
    float shade=exp(-max(0.,depth-1.)/(1.5+pulse*4.));
    float uneven=.7+.3*sin(rp.x*.13+rawMove*2.);
    inkPattern=rowPrint(shade*uneven*.55,px);
    rowMask=max(rowMask,currentRow*.85);
  }else if(u_rowEffect>4.5&&u_rowEffect<5.5){
    // A low ribbon of colored ink compresses to a stationary seam.
    float h=.9+pulse*2.5;
    float direction=u_selection.y<u_previous.y? -1.:1.;
    float depth=direction>0.?rp.y:u_selection.w-rp.y;
    float line=exp(-pow((depth-2.-sin(uv.x*8.+rawMove*3.)*pulse*.8)/h,2.));
    inkPattern=rowPrint(line*(.45+pulse*.40),px);
    selectionInk=mix(u_selectionInkColor,u_uiAccent,.74+.18*sin(uv.x*5.));
    rowMask=max(rowMask,currentRow*.85);
  }
  if(u_rowEffect>6.5){
    // Independent printing/erasing masks, never an overlay sliding across text.
    float oldRow=maskBox(px,u_previous,rowRadius)*clip*body;
    float arrive=rowPrint(move,px),depart=rowPrint(1.-move,px);
    rowMask=max(currentRow*arrive,oldRow*depart);
  }
  color=mix(color,rowColor,rowMask);alpha=max(alpha,rowMask);
  color=mix(color,selectionInk,inkPattern*currentRow*clamp(u_rowAmount,0.,1.)*.65);
  if(u_rowEffect>5.5&&u_rowEffect<6.5){
    // Horizontal leading edge follows up/down movement, with a fading wake behind it.
    float dir=u_selection.y<u_previous.y?-1.:1.;
    float oldEdge=u_previous.y+(dir>0.?u_previous.w:0.);
    float newEdge=u_selection.y+(dir>0.?u_selection.w:0.);
    float edge=mix(oldEdge,newEdge,move)+dir*1.5;
    float across=smoothstep(0.,14.,rp.x)*smoothstep(0.,14.,u_selection.z-rp.x);
    float behind=(edge-px.y)*dir;
    float wake=exp(-max(0.,behind)/(1.5+pulse*9.))*step(-1.5,behind);
    float finalEdge=exp(-pow((px.y-newEdge-dir*1.5)/1.3,2.));
    float outside=(1.-currentRow)*body*clip;
    float ribbon=across*max(wake*pulse*.8,finalEdge*.32)*outside;
    color=mix(color,u_uiAccent,rowPrint(ribbon,px)*u_rowAmount);
  }
  float surface=printed(edgeTone(px,u_panel)*u_amount,px)*u_targets.w*body;
  bool topEdge=u_inputStyle<.5||(u_inputStyle>2.5&&u_inputStyle<3.5);
  float queryDepth=topEdge?px.y-u_query.y:u_query.y+u_query.w-px.y;
  float inputWave=u_inputStyle>2.5?0.:sin(px.x*.04-u_phase*2.)*(1.+u_tap*5.+u_loading*2.);
  float inputDensity=(u_inputStyle>1.5&&u_inputStyle<2.5)?exp(min(0.,box(px,u_query,u_query.w*.5))/7.):exp(-max(0.,queryDepth-3.-inputWave)/7.);
  surface=max(surface,printed(inputDensity*min(.65,u_amount*1.8),px)*u_targets.z*query*base);
  // Unified Activity has no separate query rectangle: the input region is supplied separately by the host.
  color=mix(color,ink,surface*.48*(1.-rowMask));
  if(u_edgeSignal>.5){
    float edge=liveEdge(px,u_panel)*body*(1.-rowMask);
    vec3 edgeColor=signalInk((px-u_panel.xy)/max(vec2(1.),u_panel.zw)*2.-1.,u_phase);
    float edgeCoverage=printed(edge,px)*(.7+u_amount*.8);
    color=mix(color,edgeColor,clamp(edgeCoverage,0.,.8));
  }
  // These surface motifs are independent of the indicator: the dithered orb can stay.
  if(u_language>.5){
    vec2 corner=u_panel.xy+vec2(u_panel.z-6.,u_panel.w-6.);
    float rc=length(px-corner);
    float rings=max(1.-smoothstep(.35,1.1,abs(rc-19.)),1.-smoothstep(.35,1.1,abs(rc-25.)));
    float edgeLine=1.-smoothstep(.25,.9,abs(box(px,u_panel,bodyRadius)+4.));
    float bottom=step(u_panel.y+u_panel.w-42.,px.y);
    float ornament=rings*bottom*body;
    if(u_language>1.5){
      vec2 cp=px-u_panel.xy-vec2(11.);
      float cornerRing=max(1.-smoothstep(.25,1.,abs(length(cp)-18.)),1.-smoothstep(.25,1.,abs(length(cp)-25.)));
      ornament=max(ornament,cornerRing*body*step(px.y,u_panel.y+43.)*step(px.x,u_panel.x+43.));
    }
    color=mix(color,ink,clamp(ornament*.22+edgeLine*body*.10,0.,.28));
    vec2 mark=px-(row.xy+vec2(row.z-12.,row.w*.5));
    float markLine=1.-smoothstep(.3,1.,abs(length(mark)-3.5));
    color=mix(color,ink,markLine*rowMask*.6);
  }
  // Commit feedback can leave the avatar and originate at the pressed row.
  if(u_dispatchTarget>.5 && u_releaseRow.w>.5){
    float progress=clamp(u_releaseAge/.34,0.,1.);
    float envelope=sin(progress*3.14159);
    float sd=box(px,u_releaseRow,u_releaseRadius);
    float coverage;
    if(u_releaseStyle<.5)coverage=exp(-pow((sd+2.-progress*5.)/1.4,2.))*envelope*.65;
    else if(u_releaseStyle<1.5){
      float front=(px.x-u_releaseRow.x)/max(1.,u_releaseRow.z);
      coverage=exp(-pow((front-progress*1.4+.2)/.17,2.))*envelope*.55*step(sd,0.);
    }else coverage=envelope*.28*step(sd,0.);
    vec3 commitInk=u_dark>.5?paper:ink;
    commitInk=mix(commitInk,pigment((px-u_releaseRow.xy)/max(u_releaseRow.zw,vec2(1.)),u_releasePhase),step(.5,u_color));
    float mark=printed(coverage,px)*body*clip*.62;
    color=mix(color,commitInk,mark);
  }
  float avatarRelease=1.-step(.5,u_dispatchTarget);
  float releaseEnergy=u_burst*u_expression*avatarRelease*(1.-step(3.5,u_releaseMode));
  bool gather=u_releaseMode<.5;
  bool bloom=u_releaseMode>.5&&u_releaseMode<1.5;
  bool sparks=u_releaseMode>1.5&&u_releaseMode<2.5;
  float recoil=clamp(u_echo-u_burst,0.,1.)*avatarRelease;
  float releaseScale=gather?-.065*min(1.3,releaseEnergy)+recoil*.045:sparks?releaseEnergy*.022:u_releaseMode>2.5&&u_releaseMode<3.5?releaseEnergy*.035:0.;
  float radius=max(1.,u_signal.z)*(1.+sin(u_phase*.7)*.009*u_breathe+releaseScale);
  vec2 v=(px-u_signal.xy)/radius;
  float a=atan(v.y,v.x),r=length(v);
  float pokeLength=length(u_poke);
  vec2 pokeDirection=u_poke/max(.001,pokeLength);
  float poke=pow(max(0.,dot(v/max(.001,r),pokeDirection)),8.)*min(.52,pokeLength*u_expression*.7);
  vec2 secondary=(u_typingStyle>1.5&&u_typingStyle<3.5)?u_poke2:vec2(0.);
  float secondLength=length(secondary);
  poke+=pow(max(0.,dot(v/max(.001,r),secondary/max(.001,secondLength))),7.)*min(.3,secondLength*u_expression*.5);
  poke*=u_typingStyle<3.5?mod(u_typingStyle,2.):0.;
  vec2 posed=v/(1.+poke);
  // Traveling cap wave on a sphere: angular distance from one off-center point.
  float capHeight=0.;
  if(u_surfaceWave.x>0.){
    float capPhase=u_surfaceWave.y< -1.5?u_time*4./max(.4,u_surfaceExpression.y):u_surfaceWave.y<0.?u_phase:u_surfaceWave.y;
    capHeight=surfaceHeight(v,mod(capPhase,4.),u_surfaceWave.x);
  }
  if(u_releaseMode>3.5 && avatarRelease>.5){
    float duration=max(.4,u_surfaceExpression.y);
    // A retrigger adds a new wave while the previous wave finishes.
    capHeight+=surfaceHeight(v,clamp(u_releaseAge/duration,0.,1.)*4.,u_surfaceExpression.x);
    capHeight+=surfaceHeight(v,clamp(u_priorReleaseAge/duration,0.,1.)*4.,u_surfaceExpression.x);
  }
  if(u_loadingMotion>13.5&&u_loading>.0001){
    capHeight+=surfaceHeight(v,mod(u_time*4./max(.4,u_surfaceExpression.y),4.),u_surfaceExpression.x)*clamp(u_loading,0.,1.);
  }
  float loopHeight=surfaceHeightAt(v,mod(u_time*4./max(.4,u_loopRipple.y),4.),u_loopRipple.x,u_loopOrigin,u_loopProfile)*clamp(u_loading,0.,1.);
  posed/=1.+capHeight*u_surfaceInfluence.x+loopHeight*u_loopInfluence.x;
  float distance=signalDistance(posed,u_phase);
  float edgeActivity=.12+min(1.,u_tap+u_loading*.6+u_burst+u_wake+u_momentum*.4);
  distance-=u_shapeNoise*edgeActivity*(.65*sin(a*3.+u_phase*1.4)+.35*sin(a*5.-u_phase*.9));
  float orbMask=(1.-smoothstep(-.025,.025,distance))*u_enableOrb;
  // A faint continuous edge defines the silhouette even where the print is sparse.
  float boundaryAmount=u_boundary<.5?0.:u_boundary<1.5?exp(-pow((distance+.055)/.075,2.))*orbMask:exp(-pow((distance-.05)/.11,2.))*(1.-orbMask)*u_enableOrb;
  float edgeShade=boundaryAmount*.10;
  color=ink*edgeShade+color*(1.-edgeShade);alpha=edgeShade+alpha*(1.-edgeShade);
  vec2 displacement=(u_poke*exp(-pow(dot(v,vec2(-pokeDirection.y,pokeDirection.x)),2.)*2.6)+secondary*exp(-dot(v,v)*.7))*.65*u_expression;
  float flow=dot(v,u_poke+secondary)*.15;
  // Pressure folds a local patch; light-kick moves only the underlying volumes.
  if(u_typingStyle>3.5&&u_typingStyle<4.5){
    vec2 origin=pokeDirection*.36;
    vec2 fromOrigin=v-origin;
    displacement=fromOrigin*exp(-dot(fromOrigin,fromOrigin)*3.)*pokeLength*1.8*u_expression;
  }
  if(u_typingStyle>4.5)displacement=vec2(0.);
  if(u_typingStyle>5.5){
    float kick=min(1.2,u_tap)*u_expression;
    vec2 noise=vec2(sin(v.y*4.2+u_phase*2.+u_seed*5.),cos(v.x*3.8-u_phase*1.7+u_seed*8.));
    displacement=noise*kick*.24;
    flow+=sin(v.x*3.+v.y*2.+u_seed*7.)*kick*.12;
  }
  if(gather)displacement-=v*releaseEnergy*.10;
  float loadingWave=0.;
  float agitationWeight=u_mixing>.5?u_effectMix.z:float(u_loadingMotion>6.5&&u_loadingMotion<7.5);
  float eddyWeight=u_mixing>.5?u_effectMix.y:float(u_loadingMotion>11.5&&u_loadingMotion<12.5);
  float driftWeight=u_mixing>.5?u_effectMix.x:float(u_loadingMotion>4.5&&u_loadingMotion<5.5);
  float waveMode=u_mixCustom>.5?u_mixWaveMode:u_loadingMotion;
  float wavePhase=u_mixCustom>.5?u_mixWave.z:u_phase;
  float waveLoading=u_mixCustom>.5?u_mixWave.x*u_mixWaveWeight:u_loading;
  float waveCoupling=u_mixCustom>.5?u_mixWave.y:u_coupling;
  float agitationLoading=u_mixCustom>.5?u_mixAgitation.x:u_loading;
  float agitationPhase=u_mixCustom>.5?u_mixAgitation.z:u_phase;
  float agitationCoupling=u_mixCustom>.5?u_mixAgitation.y:u_coupling;
  float eddyLoading=u_mixCustom>.5?u_mixEddies.x:u_loading;
  float eddyPhase=u_mixCustom>.5?u_mixEddies.z:u_phase;
  float eddyCoupling=u_mixCustom>.5?u_mixEddies.y:u_coupling;
  float driftLoading=u_mixCustom>.5?u_mixDrift.x:u_loading;
  float driftTransport=u_mixCustom>.5?u_mixDrift.w:u_transport;
  float driftCoupling=u_mixCustom>.5?u_mixDrift.y:u_coupling;
  if((waveMode>.5&&waveMode<2.5)||(waveMode>3.5&&waveMode<4.5)||(waveMode>7.5&&waveMode<11.5)){
    float cycle=wavePhase*2.3;
    float travel=fract(cycle);
    vec2 origin=vec2(mod(floor(cycle),2.)<1.?-1.:1.,0.);
    if(waveMode>3.5)origin=vec2(1.3,0.);
    if(waveMode>7.5&&waveMode<8.5)origin=vec2(-1.3,0.);
    if(waveMode>8.5&&waveMode<9.5)origin=vec2(0.,-1.3);
    if(waveMode>9.5&&waveMode<10.5)origin=vec2(0.,1.3);
    if(waveMode>10.5){float heading=sin(floor(cycle)*127.1+u_seed*311.7)*43758.5453;float angle=fract(heading)*6.28318;origin=vec2(cos(angle),sin(angle))*1.3;}
    vec2 normal=normalize(v-origin+vec2(.0001));
    float front=length(v-origin)-travel*2.9;
    if(waveMode>1.5&&waveMode<2.5){normal=normalize(vec2(origin.x,.42*sin(cycle)));front=dot(v,normal)+1.3-travel*2.6;}
    loadingWave=exp(-pow(front/.28,2.))*sin(travel*3.14159);
    displacement+=normal*loadingWave*waveLoading*.32*waveCoupling;
    // A traveling wave tips the light and squeezes the volume as it passes.
    light+=normal*loadingWave*waveLoading*.48*max(0.,waveCoupling-1.);
    flow+=sin(dot(v,normal)*3.-cycle*2.)*loadingWave*waveLoading*.12*max(0.,waveCoupling-1.);
    flow+=waveLoading*loadingWave*.24;
  }else if(u_mixing<.5&&u_loadingMotion<3.5){
    float strength=u_loadingMotion>2.5?.23:.08;
    displacement+=u_loading*strength*vec2(sin(v.y*3.+u_phase*3.),cos(v.x*2.7-u_phase*2.3));
    flow+=u_loading*strength*sin(v.y*3.+u_phase*2.);
  }
  if(agitationWeight>0.){
    // Agitation adds shear and pressure rather than another spinning overlay.
    vec2 turbulence=agitationField(v,agitationPhase)*agitationWeight;
    displacement+=agitationLoading*turbulence*.23*agitationCoupling;
    light+=turbulence*agitationLoading*.18*max(0.,agitationCoupling-1.);
    flow+=agitationWeight*agitationLoading*.15*sin(v.x*3.+v.y*2.-agitationPhase*3.);
  }
  if(eddyWeight>0.){
    displacement+=eddyOffset(v,eddyPhase)*eddyLoading*eddyCoupling*eddyWeight;
    light+=vec2(cos(eddyPhase*2.3),sin(eddyPhase*1.7))*eddyLoading*.22*eddyCoupling*eddyWeight;
  }
  displacement+=u_momentum*.11*vec2(sin(v.y*2.7+u_phase*1.3),cos(v.x*3.1-u_phase));
  if(driftWeight>0.){
    displacement+=driftOffset(v,driftTransport)*driftLoading*driftCoupling*driftWeight;
    light+=vec2(sin(driftTransport),cos(driftTransport*.7))*.22*driftLoading*max(0.,driftCoupling-1.)*driftWeight;
  }
  if(u_loadingMotion>12.5&&u_loadingMotion<13.5){
    vec2 q=v-vec2(u_waveOffset,-u_waveOffset*.55);
    float rr=length(q);
    float ring=sin(rr*7.-u_phase*3.2)*exp(-rr*.65);
    vec2 normal=q/max(.05,rr);
    displacement-=normal*ring*u_loading*.18*u_coupling;
    flow+=ring*u_loading*.09;
    light+=normal*ring*u_loading*.15;
    loadingWave=abs(ring);
  }
  // Independent experimental operations; all zero in existing expressions.
  displacement+=u_primitives.x*.24*vec2(sin(u_phase*.9),cos(u_phase*.67));
  vec2 pq=v-vec2(.2*sin(u_phase),.2*cos(u_phase*.83));
  displacement+=u_primitives.y*vec2(-pq.y,pq.x)*exp(-dot(pq,pq)*1.8)*sin(u_phase*1.6)*.65;
  displacement+=u_primitives.z*.19*vec2(sin(v.y*4.+u_phase*2.7),cos(v.x*3.4-u_phase*2.1));
  displacement+=vec2(0.,u_gravity*.30*(1.-smoothstep(.15,1.1,r)));
  float driftAngle=u_mixCustom>.5?u_mixDriftAngle:u_driftAngle;
  vec2 driftDirection=vec2(cos(driftAngle),sin(driftAngle));
  displacement+=v*(capHeight*u_surfaceInfluence.y+loopHeight*u_loopInfluence.y);
  vec2 sampleV=v-displacement;
  vec2 samplePx=px-displacement*radius+driftDirection*driftTransport*radius*(u_mixing>.5?driftWeight:1.);
  float activity=1.+u_wake*.5;
  float detail=u_idleAmount*activity;
  float dome=sqrt(max(0.,1.-min(1.,dot(sampleV,sampleV))));
  flow+=1.4*(capHeight*u_surfaceInfluence.z+loopHeight*u_loopInfluence.z);
  flow+=sin(sampleV.x*2.8+u_phase*1.4)*cos(sampleV.y*3.1-u_phase)*.09*detail;
  float tone=.40+.34*dot(sampleV,light)*u_lightStrength+.25*dome*u_volume+flow+releaseEnergy*.14;
  if(u_interior>.5){
    // Two unequal, softly lit volumes wander INSIDE one stable silhouette.
    // Incommensurate, seed-offset paths never lock into a single perimeter orbit.
    float t=u_phase,seed=u_seed*6.28318;
    float roam=.24+.10*min(2.,detail);
    vec2 c1=roam*vec2(sin(t*.91+seed)+.28*sin(t*1.73+2.),cos(t*.67+seed*.7)+.2*sin(t*1.37));
    vec2 c2=roam*vec2(cos(t*1.19+2.1+seed)-.22*sin(t*.83),sin(t*.79+4.+seed*.4)+.3*cos(t*1.57));
    c1+=u_poke*.24*u_expression;
    c2-=(u_poke*.18+u_poke2*.12)*u_expression;
    if(u_typingStyle>4.5){c1+=u_poke*.35;c2-=u_poke*.28;}
    if(gather){
      c1*=1.-min(.78,releaseEnergy*.60);
      c2*=1.-min(.78,releaseEnergy*.60);
    }
    vec2 q1=(sampleV-c1)/.76,q2=(sampleV-c2)/.39;
    float h1=exp(-dot(q1,q1)*1.5),h2=exp(-dot(q2,q2)*1.5);
    float shade1=(.8+.30*dot(q1,light))*h1;
    float shade2=(.75-.26*dot(q2,light))*h2;
    tone=.16+.70*shade1+.47*shade2+flow*.65+releaseEnergy*.1;
  }
  // Bloom is illumination passing through the volume, with no outer ripple.
  float bloomFront=length(sampleV-vec2(-.22,.18))-clamp(u_releaseAge/.38,0.,1.)*1.65;
  float bloomLight=bloom?exp(-pow(bloomFront/.46,2.))*releaseEnergy:0.;
  tone+=bloomLight*.40;
  if(driftWeight>0.)tone+=sin(sampleV.x*3.4+driftTransport*3.4)*driftLoading*.10*driftWeight;
  tone=clamp((tone-.5)*u_contrast+.5,.015,.985);
  // Fill changes coverage, independently of shape, palette and motion.
  float fill=u_fillWake>.5?mix(.025,u_fill,smoothstep(0.,1.,u_appearance)):u_fill;
  tone=fill<.5?tone*fill*2.:mix(tone,1.,(fill-.5)*2.);
  // Smaller avatars use a broader contrast range, preserving the same silhouette.
  tone=mix(tone,smoothstep(.10,.90,tone),1.-smoothstep(18.,35.,radius));
  vec3 orbInk=signalInk(sampleV,u_phase);
  if(u_primitives.w>0.)orbInk=mix(orbInk,pigment(sampleV,u_phase),u_primitives.w*(.5+.5*sin(sampleV.x*2.+u_phase)));
  if(bloom)orbInk=mix(orbInk,pigment(sampleV.yx,u_releasePhase+2.4),clamp(bloomLight*.7,0.,.8)*step(.5,u_color));
  if(u_shape>2.5&&u_shape<3.5){
    vec2 offset=twinOffset(u_phase);
    float first=1.-smoothstep(-.05,.05,blobDistance((posed-offset)/.74,u_phase));
    float second=1.-smoothstep(-.05,.05,blobDistance((posed+offset)/.72,-u_phase+1.7));
    float overlap=first*second;
    tone=clamp(tone+overlap*.17,.04,.98);
    vec3 secondInk=signalInk(-v+vec2(.5,-.3),u_phase+2.2);
    orbInk=mix(orbInk,secondInk,second*.48);
    orbInk=mix(orbInk,orbInk*(u_dark>.5?1.15:.78),overlap*.45);
  }
  if((u_mixCustom>.5?u_mixWaveAccent:u_rippleAccent)>.5){
    vec3 accent=u_color>5.5?vec3(.91,.53,.24):vec3(.29,.68,.86);
    orbInk=mix(orbInk,accent,clamp(loadingWave*(u_mixCustom>.5?waveLoading:u_loading)*.8,0.,.85));
  }
  if(u_toneSteps>1.&&!(u_debugStage>.5&&u_debugStage<5.5)){
    float level=floor(clamp(tone,0.,.999)*u_toneSteps)/(u_toneSteps-1.);
    // Lightness bands retain the current hue; not a set of unrelated colors.
    orbInk=mix(paper,orbInk,.30+.70*level);
    tone=mix(tone,level,.5);
  }
  if(u_debugStage>.5&&u_debugStage<5.5){
    // Walkthrough isolates the actual terms; no hidden phase offset, flow or displacement.
    float tutorialDome=sqrt(max(0.,1.-min(1.,dot(v,v))));
    tone=.40;
    if(u_debugStage>1.5)tone+=.25*tutorialDome*u_volume;
    if(u_debugStage>2.5)tone+=.34*dot(v,vec2(cos(u_lightAngle),sin(u_lightAngle)))*u_lightStrength;
    tone=clamp((tone-.5)*u_contrast+.5,.015,.985);
    tone=u_fill<.5?tone*u_fill*2.:mix(tone,1.,(u_fill-.5)*2.);
    if(u_debugStage<4.5)orbInk=ink;
    else orbInk=signalInk(v,u_phase);
    if(u_debugStage>4.5&&u_toneSteps>1.){float level=floor(clamp(tone,0.,.999)*u_toneSteps)/(u_toneSteps-1.);orbInk=mix(paper,orbInk,.30+.70*level);tone=mix(tone,level,.5);}
    samplePx=px;
  }
  // Isolated recipe components use the same coordinates and phase as the complete field.
  if(u_debugStage>5.5&&u_debugStage<6.5){tone=dome;orbInk=ink;}
  if(u_debugStage>6.5&&u_debugStage<7.5){tone=clamp(.5+.5*dot(sampleV,light),0.,1.);orbInk=ink;}
  if(u_debugStage>7.5&&u_debugStage<8.5){tone=.5+sin(sampleV.x*2.8+u_phase*1.4)*cos(sampleV.y*3.1-u_phase)*.09*detail;orbInk=ink;}
  float printAmount=(u_debugStage>.5&&u_debugStage<3.5)||u_debugStage>5.5?0.:u_targets.x;
  float coverage=orbMask*mix(1.,printed(tone,samplePx),printAmount);
  vec3 orbColor=mix(paper,orbInk,mix(tone,1.,printAmount));
  color=orbColor*coverage+color*(1.-coverage);alpha=coverage+alpha*(1.-coverage);
  // Expanding isocontours use the SAME shape function as the core, including twins.
  for(int i=0;i<2;i++){
    float age=i==0?u_releaseAge:u_priorReleaseAge;
    float phase=i==0?u_releasePhase:u_priorReleasePhase;
    float p=clamp(age/.36,0.,1.);
    float scale=1.02+(1.-pow(1.-p,2.))*.24;
    float contour=signalDistance((px-u_signal.xy)/(max(1.,u_signal.z)*scale),phase)*scale;
    float halo=exp(-pow(contour/.065,2.))*smoothstep(0.,.05,age)*pow(1.-p,1.3)*.85*u_enableOrb*(1.-orbMask)*avatarRelease*step(2.5,u_releaseMode)*(1.-step(3.5,u_releaseMode));
    float haloInk=mix(halo,printed(halo,px),u_targets.x);
    vec3 haloColor=mix(ink,pigment(v,phase+p*3.),step(.5,u_color));
    color=haloColor*haloInk+color*(1.-haloInk);alpha=haloInk+alpha*(1.-haloInk);
  }
  // A few short, printed flecks escape the contour; no ring and no particle system.
  if(sparks&&u_releaseAge<.32){
    float p=clamp(u_releaseAge/.32,0.,1.);
    float envelope=pow(sin(p*3.14159),.7)*avatarRelease*u_enableOrb;
    vec2 local=(px-u_signal.xy)/max(1.,u_signal.z);
    float flecks=0.;
    for(int i=0;i<18;i++){
      if(float(i)>=u_sparkCount)break;
      float angle=float(i)*6.28318/max(1.,u_sparkCount)+u_seed*6.28318+sin(u_releasePhase+float(i)*2.1)*.34;
      vec2 dir=vec2(cos(angle),sin(angle));
      vec2 center=dir*(1.025+p*.30);
      vec2 offset=local-center;
      float along=dot(offset,dir),across=dot(offset,vec2(-dir.y,dir.x));
      float fleck=exp(-pow(along/(.075-p*.025),2.)-pow(across/.037,2.));
      flecks=max(flecks,fleck*envelope*.85);
    }
    flecks*=1.-orbMask;
    float marks=printed(flecks,px);
    vec3 sparkInk=mix(ink,pigment(local,u_releasePhase),step(.5,u_color));
    color=sparkInk*marks+color*(1.-marks);alpha=marks+alpha*(1.-marks);
  }
  float distanceFromSignal=length((px-u_signal.xy)/max(u_panel.zw,vec2(u_signal.z*2.)));
  float appearing=clamp(u_appearance*2.-distanceFromSignal*.75,0.,1.);
  float visible=step(rank8(floor(px/u_pitch)),appearing);if(u_appearance>=1.)visible=1.;if(u_appearance<=0.)visible=0.;
  if(u_fillWake>.5)visible=1.;
  fragColor=vec4(color*visible,alpha*visible);
}`;
