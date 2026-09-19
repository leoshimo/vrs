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
  activityColor?: SignalColor;
  entranceRipple?: boolean;
  idleSwirl?: number;
  workingOrbit?: number;
  orbitRate?: number;
  swirlRate?: number;
  typingSparks?: boolean;
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
  lightRotationRate?: number;
  flowRate?: number;
  typingEnergy?: number;
  pressureSpread?: number;
  pressureOffset?: number;
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
  activityColor: "mono",
  entranceRipple: false,
  idleSwirl: 0,
  workingOrbit: 0,
  orbitRate: 1,
  swirlRate: 0.8,
  typingSparks: false,
  loadingMotion: "alternating",
  edgeSignal: "off",
  edgeMotion: "wave",
  typingMotion: "light",
  interior: "light",
  idleAmount: 1.2,
  typingEnergy: 1.15,
  pressureSpread: 3,
  pressureOffset: 0.36,
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

// Field, lighting, and printing share coordinates in the avatar's local space.
export const signalFragment = `#version 300 es
precision highp float;
uniform vec2 u_resolution;
uniform float u_pixelRatio,u_phase,u_breathe,u_texture,u_dark,u_appearance;
uniform float u_pitch,u_expression,u_enableOrb,u_idleAmount,u_fill,u_contrast,u_matrix;
uniform float u_color,u_saturation,u_activitySaturation,u_colorStrength;
uniform float u_volume,u_lightStrength,u_lightAngle,u_toneSteps,u_debugStage,u_print,u_fillWake;
uniform vec2 u_lavaPhase,u_printOffset;
uniform vec3 u_paperColor,u_inkColor;
uniform vec4 u_signal;
uniform vec4 u_history[128],u_randomHistory[128],u_accents[4];
uniform vec4 u_historySpans;
uniform highp sampler2D u_waveCurves;
uniform vec4 u_waveProfiles[8],u_waveInfluences[8];
uniform vec4 u_effects[32],u_effectOptions[32];
uniform float u_effectCount,u_sparkCount;
uniform vec4 u_recipeDisplacement;
uniform vec2 u_recipePoke;
out vec4 fragColor;
${displacementEffectsGLSL}
float waveAt(float delay,int row,float span){
  float position=clamp(delay/max(.01,span)*127.,0.,127.);
  float i=floor(position),y=(float(row)+.5)/8.;
  float a=texture(u_waveCurves,vec2((i+.5)/128.,y)).r;
  float b=texture(u_waveCurves,vec2((min(127.,i+1.)+.5)/128.,y)).r;
  return mix(a,b,fract(position))*step(0.,delay)*step(delay,span);
}
vec4 responseAt(float delay){
  vec4 position=clamp(delay*127./max(vec4(.01),u_historySpans),0.,127.);
  vec4 outValue=vec4(0.);
  for(int channel=0;channel<4;channel++){
    int i=int(floor(position[channel]));
    outValue[channel]=mix(u_history[i][channel],u_history[min(127,i+1)][channel],fract(position[channel]))*step(0.,delay)*step(delay,u_historySpans[channel]);
  }
  return outValue;
}
vec4 variationAt(float delay){
  float position=clamp(delay*127./max(.01,u_historySpans.y),0.,127.);
  int i=int(floor(position));
  return mix(u_randomHistory[i],u_randomHistory[min(127,i+1)],fract(position));
}
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
vec3 paletteInkSaturation(vec2 p,float phase,float palette,float saturation){
  float t=.5+.5*sin(p.x*1.8+p.y*.9+phase*.7);
  float s=.5+.5*cos(p.y*2.-p.x*.8-phase*.5);
  vec3 a=palette<1.5?vec3(.10,.68,.72):vec3(.95,.29,.15);
  vec3 b=palette<1.5?vec3(.43,.36,.94):vec3(.96,.63,.22);
  vec3 c=palette<1.5?vec3(.95,.40,.68):vec3(.71,.23,.54);
  vec3 col=mix(mix(a,b,t),c,s*.48);
  if(palette>2.5&&palette<3.5)col=mix(vec3(.08,.42,.62),vec3(.85,.91,.68),smoothstep(.38,.62,t));
  if(palette>3.5&&palette<4.5)col=mix(vec3(.80,.27,.17),vec3(.12,.65,.59),smoothstep(.38,.62,t));
  if(palette>4.5){
    float band=floor(clamp(t*.7+s*.3,0.,.999)*3.);
    col=band<1.?vec3(.08,.68,.79):band<2.?vec3(.91,.25,.52):vec3(.96,.81,.29);
  }
  if(palette>5.5){
    vec3 x=vec3(.18,.52,.89), y=vec3(.52,.77,.60), z=vec3(.97,.59,.27);
    if(palette>6.5&&palette<7.5){x=vec3(.52,.68,.90);y=vec3(.76,.61,.83);z=vec3(.93,.80,.61);}
    if(palette>7.5){x=vec3(.27,.37,.87);y=vec3(.76,.41,.61);z=vec3(.94,.70,.40);}
    col=t<.5?mix(x,y,smoothstep(0.,.5,t)):mix(y,z,smoothstep(.5,1.,t));
  }
  if(palette>8.5&&palette<12.5){
    col=palette<9.5?vec3(.39,.52,.64):palette<10.5?vec3(.43,.57,.45):palette<11.5?vec3(.62,.46,.59):vec3(.68,.55,.31);
    col*=.82+t*.25;
  }
  if(palette>12.5)col=palette<13.5?mix(vec3(.53,.67,.70),vec3(.77,.73,.66),t):mix(vec3(.54,.51,.68),vec3(.76,.63,.56),t);
  float luminance=dot(col,vec3(.2126,.7152,.0722));
  col=mix(vec3(luminance),col,saturation);
  return mix(col*.7,col+.12,u_dark);
}

vec3 signalInk(vec2 p,float phase){
  vec3 base=mix(u_inkColor,paletteInkSaturation(p,phase,u_color,u_saturation),step(.5,u_color)*u_colorStrength);
  vec3 tint=vec3(0.);float strength=0.;
  for(int i=0;i<4;i++){tint+=paletteInkSaturation(p,phase,u_accents[i].x,u_activitySaturation)*u_accents[i].y;strength+=u_accents[i].y;}
  if(strength>.0001){tint/=strength;float l=dot(tint,vec3(.2126,.7152,.0722));tint=mix(tint,tint+(base-vec3(l))*.65,.7);}
  return mix(base,clamp(tint,0.,1.),min(.8,strength));
}
void main(){
  vec2 px=vec2(gl_FragCoord.x,u_resolution.y-gl_FragCoord.y)/u_pixelRatio;
  vec3 paper=u_paperColor,ink=u_inkColor,color=vec3(0.);float alpha=0.;
  vec2 lavaPhase=u_lavaPhase;
  float lightAngle=lavaPhase.x-.7+.24*sin(lavaPhase.x*1.7)+u_lightAngle;
  vec2 light=vec2(cos(lightAngle),sin(lightAngle));
  float gather=0.;
  for(int i=0;i<32;i++){if(float(i)>=u_effectCount)break;if(u_effects[i].x==10.)gather+=u_effects[i].y*u_expression;}
  float radius=max(1.,u_signal.z)*(1.+sin(u_phase*.7)*.009*u_breathe-.065*min(1.3,gather));
  vec2 v=(px-u_signal.xy)/radius;
  float r=length(v);
  vec3 surface=vec3(0.);
  vec3 point=normalize(vec3(v,sqrt(max(0.,1.-dot(v,v)))));
  for(int channel=0;channel<8;channel++){
    if(u_waveInfluences[channel].w<.5)continue;
    vec4 profile=u_waveProfiles[channel];
    vec3 origin=normalize(vec3(profile.xy,.84));
    float delay=acos(clamp(dot(point,origin),-1.,1.))/3.14159*max(.15,profile.z);
    surface+=waveAt(delay,channel,profile.w)*.18*u_waveInfluences[channel].xyz;
  }
  vec2 posed=v/(1.+surface.x);
  float distance=length(posed)-1.;
  float orbMask=(1.-smoothstep(-.025,.025,distance))*u_enableOrb;
  vec2 displacement=-v*gather*.10;
  float flow=gather*.14;
  // Each channel keeps its own parameters and phase through composition.
  for(int i=0;i<32;i++){
    if(float(i)>=u_effectCount)break;
    vec4 effect=u_effects[i],options=u_effectOptions[i];
    float kind=effect.x,amount=effect.y,t=effect.z,coupling=effect.w;
    if(kind==1.){
      vec2 direction=vec2(cos(u_phase*1.3),sin(u_phase*1.3));
      vec2 q=v-direction*clamp(options.y,0.,.95);
      displacement+=q*exp(-dot(q,q)*max(.5,options.x))*amount*1.8*u_expression;
      flow+=dot(v,direction)*amount*.15;
    }else if(kind==2.){
      vec2 poke=vec2(.25,.10)*amount;
      light+=poke*.65*u_expression;flow+=dot(v,poke)*.15;
    }else if(kind==3.){
      float kick=min(1.2,amount)*u_expression;
      vec2 noise=vec2(sin(v.y*4.2+u_phase*2.+.37*5.),cos(v.x*3.8-u_phase*1.7+.37*8.));
      displacement+=noise*kick*.24;flow+=sin(v.x*3.+v.y*2.+.37*7.)*kick*.12;
    }else if(kind==4.){
      vec2 turbulence=agitationField(v,t);
      displacement+=amount*turbulence*.23*coupling;
      light+=turbulence*amount*.18*max(0.,coupling-1.);
      flow+=amount*.15*sin(v.x*3.+v.y*2.-t*3.);
    }else if(kind==5.){
      displacement+=eddyOffset(v,t)*amount*coupling;
      light+=vec2(cos(t*2.3),sin(t*1.7))*amount*.22*coupling;
    }else if(kind==6.){
      displacement+=driftOffset(v,t)*amount*coupling;
      light+=vec2(sin(t),cos(t*.7))*.22*amount*max(0.,coupling-1.);
      flow+=sin(v.x*3.4+t*3.4)*amount*.10;
    }else if(kind==7.){
      float cycle=t*2.3,travel=fract(cycle);
      vec2 origin=vec2(mod(floor(cycle),2.)<1.?-1.:1.,0.);
      if(options.z==1.)origin=vec2(1.,0.);
      else if(options.z==2.)origin=vec2(-1.,0.);
      else if(options.z==3.)origin=vec2(0.,1.);
      else if(options.z==4.)origin=vec2(0.,-1.);
      vec2 normal=normalize(v-origin+vec2(.0001));
      float front=length(v-origin)-travel*2.9;
      float crest=exp(-pow(front/.28,2.))*sin(travel*3.14159);
      displacement+=normal*crest*amount*.32*coupling;
      light+=normal*crest*amount*.48*max(0.,coupling-1.);
      flow+=sin(dot(v,normal)*3.-cycle*2.)*crest*amount*.12*max(0.,coupling-1.)+amount*crest*.24;
    }else if(kind==8.||kind==9.){
      float angle=atan(v.y,v.x)-t;
      float crest=kind==8.?sin(angle+.32*sin(t*.73))+.3*sin(2.*angle-t*.31):pow(.5+.5*cos(angle),9.);
      float ring=exp(-pow((r-.78)/.19,2.))*crest*amount;
      displacement+=vec2(-v.y,v.x)*ring*.16;flow+=ring*.28*(1.-options.w);
    }
  }
  // Recipe probes expose the same displacement operations at a chosen phase.
  displacement+=u_recipeDisplacement.x*.24*vec2(sin(u_phase*.9),cos(u_phase*.67));
  vec2 pq=v-vec2(.2*sin(u_phase),.2*cos(u_phase*.83));
  displacement+=u_recipeDisplacement.y*vec2(-pq.y,pq.x)*exp(-dot(pq,pq)*1.8)*sin(u_phase*1.6)*.65;
  displacement+=u_recipeDisplacement.z*.19*vec2(sin(v.y*4.+u_phase*2.7),cos(v.x*3.4-u_phase*2.1));
  displacement+=vec2(0.,u_recipeDisplacement.w*.30*(1.-smoothstep(.15,1.1,r)));
  light+=u_recipePoke*.65*u_expression;
  flow+=dot(v,u_recipePoke)*.15;
  displacement+=v*surface.y;
  vec2 sampleV=v-displacement;
  vec2 samplePx=px-displacement*radius+u_printOffset*radius;
  float dome=sqrt(max(0.,1.-min(1.,dot(sampleV,sampleV))));
  flow+=1.4*surface.z;
  flow+=sin(sampleV.x*2.8+lavaPhase.y*1.4)*cos(sampleV.y*3.1-lavaPhase.y)*.09*u_idleAmount;
  float tone=.40+.34*dot(sampleV,light)*u_lightStrength+.25*dome*u_volume+flow;
  tone+=responseAt(length(sampleV-vec2(-.22,.18))*.16).w*.40;
  tone=clamp((tone-.5)*u_contrast+.5,.015,.985);
  float fill=u_fillWake>.5?mix(.025,u_fill,smoothstep(0.,1.,u_appearance)):u_fill;
  tone=fill<.5?tone*fill*2.:mix(tone,1.,(fill-.5)*2.);
  tone=mix(tone,smoothstep(.10,.90,tone),1.-smoothstep(18.,35.,radius));
  vec3 orbInk=signalInk(sampleV,u_phase);
  if(u_toneSteps>1.&&!(u_debugStage>.5&&u_debugStage<5.5)){
    float level=floor(clamp(tone,0.,.999)*u_toneSteps)/(u_toneSteps-1.);
    orbInk=mix(paper,orbInk,.30+.70*level);tone=mix(tone,level,.5);
  }
  if(u_debugStage>.5&&u_debugStage<5.5){
    // Intermediate coverage stages for Recipe.
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
  if(u_debugStage>7.5&&u_debugStage<8.5){tone=.5+sin(sampleV.x*2.8+lavaPhase.y*1.4)*cos(sampleV.y*3.1-lavaPhase.y)*.09*u_idleAmount;orbInk=ink;}
  float printAmount=(u_debugStage>.5&&u_debugStage<3.5)||u_debugStage>5.5?0.:u_print;
  float coverage=orbMask*mix(1.,printed(tone,samplePx),printAmount);
  vec3 orbColor=mix(paper,orbInk,mix(tone,1.,printAmount));
  color=orbColor*coverage+color*(1.-coverage);alpha=coverage+alpha*(1.-coverage);
    float echoDelay=(r-1.)*.9;
    float echo=responseAt(echoDelay).z*step(1.,r)*exp(-max(0.,r-1.)*6.);
    float marks=0.;
    for(int i=0;i<18;i++){
      if(float(i)>=u_sparkCount)break;
      float id=float(i),speed=.8+.5*fract(sin(id*27.31+4.)*43758.5);
      float delay=(r-1.)/(1.7*speed);
      vec4 response=responseAt(delay);vec4 noise=variationAt(delay)/max(.001,response.y);
      float jitter=sin(id*13.7+noise.x*8.+noise.y*3.);
      float angle=id*2.39996+jitter*.7+noise.z*2.;
      float across=sin(atan(v.y,v.x)-angle)*r;
      float front=cos(atan(v.y,v.x)-angle);
      float width=.014+.025*fract(sin(id*17.+noise.w)*351.);
      float mark=exp(-pow(across/width,2.))*step(.5,front)*response.y;
      marks=max(marks,mark*step(1.,r)*exp(-max(0.,r-1.)*3.));
    }
    marks=printed(max(marks,echo*.65),px)*(1.-orbMask)*u_enableOrb;
    vec3 tint=signalInk(v,u_phase);
    color=tint*marks+color*(1.-marks);alpha=marks+alpha*(1.-marks);

  float distanceFromSignal=length((px-u_signal.xy)/(u_signal.z*2.));
  float appearing=clamp(u_appearance*2.-distanceFromSignal*.75,0.,1.);
  float visible=step(rank8(floor(px/u_pitch)),appearing);
  if(u_appearance>=1.)visible=1.;if(u_appearance<=0.)visible=0.;
  if(u_fillWake>.5)visible=smoothstep(0.,.12,u_appearance);
  fragColor=vec4(color*visible,alpha*visible);
}`;
