var e = {
	appearance: {
		shape: "pearl",
		placement: "leading",
		breathe: !0,
		selection: "fill",
		texture: "bayer",
		effects: {
			indicator: !0,
			selection: !1,
			input: !1,
			container: !1,
			placeholder: !1
		},
		edge: "wave",
		amount: .22,
		pitch: 2.5,
		expression: 1.25,
		color: "mono",
		activityColor: "mono",
		toneSteps: 5,
		colorStrength: .5,
		fill: .5,
		contrast: 1.25,
		matrix: 8,
		saturation: .5,
		activitySaturation: .5,
		gravity: .1,
		volume: 1.5,
		lightStrength: .95,
		lightAngle: -180,
		uiSurface: "porcelain"
	},
	assignments: {
		idle: {
			id: "rest",
			name: "Lava",
			description: "A flowing field with rotating light.",
			patterns: [{
				kind: "lava",
				settings: {
					flowStrength: 2,
					flowSpeed: 2,
					lightRotation: 1
				}
			}],
			duration: .3,
			color: {
				mode: "none",
				palette: "spectrum",
				strength: .3,
				attack: .07,
				duration: .32
			},
			custom: !1
		},
		typing: {
			id: "field-nudge",
			name: "Nudge",
			description: "Briefly accelerates the field, then returns to its normal pace.",
			patterns: [{
				kind: "nudge",
				settings: { strength: 1.04 }
			}],
			duration: .24,
			color: {
				mode: "none",
				palette: "spectrum",
				strength: .3,
				attack: .07,
				duration: .32
			},
			custom: !1,
			momentum: !1
		},
		submit: {
			id: "field-nudge",
			name: "Nudge",
			description: "Briefly accelerates the field, then returns to its normal pace.",
			patterns: [{
				kind: "nudge",
				settings: { strength: 1.04 }
			}],
			duration: .24,
			color: {
				mode: "none",
				palette: "spectrum",
				strength: .3,
				attack: .07,
				duration: .32
			},
			custom: !1,
			momentum: !1
		},
		working: {
			id: "working",
			name: "Stir + Ripple",
			description: "Irregular flow with a repeating surface wave.",
			patterns: [{
				kind: "stir",
				settings: {
					strength: .55,
					speed: 2.45,
					distortion: 1
				}
			}, {
				kind: "ripple",
				settings: {
					strength: .28,
					travel: 5,
					width: .32,
					originX: -.3,
					originY: -.45
				}
			}],
			duration: .3,
			color: {
				mode: "accent",
				palette: "spectrum",
				strength: .3,
				attack: .07,
				duration: .32
			},
			custom: !1
		},
		open: {
			id: "printed-open",
			name: "Printed — Subtle",
			description: "",
			patterns: [{
				kind: "printed",
				settings: {}
			}],
			duration: .15,
			color: {
				mode: "none",
				palette: "spectrum",
				strength: .3,
				attack: .07,
				duration: .32
			},
			revealStart: .72,
			custom: !1
		},
		openAfterIdle: {
			id: "entrance-print-ripple",
			name: "Printed + Ripple",
			description: "Printed marks resolve as a ridge crosses the sphere.",
			patterns: [{
				kind: "printed",
				settings: {}
			}, {
				kind: "ripple",
				settings: {
					strength: .45,
					travel: .3,
					width: .32,
					originX: -.7,
					originY: -.6
				}
			}],
			duration: .3,
			color: {
				mode: "none",
				palette: "spectrum",
				strength: .3,
				attack: .07,
				duration: .32
			},
			custom: !1
		},
		complete: {
			id: "sparks",
			name: "Sparks",
			description: "Short marks escape the surface.",
			patterns: [{
				kind: "sparks",
				settings: {
					strength: .8,
					count: 11
				}
			}],
			duration: .3,
			color: {
				mode: "none",
				palette: "spectrum",
				strength: .3,
				attack: .07,
				duration: .32
			},
			custom: !1
		}
	}
};
//#endregion
//#region work/avatar/assigned-setup.ts
function t() {
	return structuredClone(e);
}
//#endregion
//#region work/avatar/effect-settings.ts
var n = {
	strength: [
		"Strength",
		0,
		2.5,
		.05,
		1
	],
	speed: [
		"Speed",
		0,
		3,
		.05,
		1
	],
	distortion: [
		"Distortion",
		0,
		3,
		.1,
		1
	]
}, r = {
	lava: {
		flowStrength: [
			"Flow strength",
			0,
			2.4,
			.1,
			1.6
		],
		flowSpeed: [
			"Flow speed",
			0,
			3,
			.05,
			1
		],
		lightRotation: [
			"Light rotation",
			0,
			3,
			.05,
			1
		]
	},
	nudge: { strength: [
		"Strength",
		0,
		1.5,
		.05,
		.6
	] },
	tilt: { strength: [
		"Strength",
		0,
		2,
		.05,
		1
	] },
	pressure: {
		strength: [
			"Strength",
			0,
			1.5,
			.05,
			.28
		],
		spread: [
			"Concentration",
			.5,
			10,
			.1,
			3
		],
		offset: [
			"Distance from center",
			0,
			.95,
			.05,
			.36
		]
	},
	ruffle: { strength: [
		"Strength",
		0,
		2,
		.05,
		.5
	] },
	stir: n,
	drift: {
		...n,
		angle: [
			"Direction",
			-180,
			180,
			1,
			20
		]
	},
	eddies: n,
	sweep: {
		...n,
		direction: [
			"Direction",
			0,
			4,
			1,
			0
		]
	},
	swirl: {
		strength: [
			"Strength",
			0,
			2,
			.05,
			.8
		],
		speed: [
			"Speed",
			0,
			3,
			.1,
			.8
		]
	},
	orbit: {
		strength: [
			"Strength",
			0,
			2,
			.05,
			1
		],
		speed: [
			"Speed",
			0,
			3,
			.1,
			1.8
		]
	},
	ripple: {
		strength: [
			"Strength",
			0,
			1.5,
			.05,
			.45
		],
		travel: [
			"Travel time",
			.15,
			15,
			.05,
			.65
		],
		width: [
			"Width",
			.1,
			.8,
			.02,
			.32
		],
		originX: [
			"Origin X",
			-1,
			1,
			.05,
			-.7
		],
		originY: [
			"Origin Y",
			-1,
			1,
			.05,
			-.6
		],
		silhouette: [
			"Silhouette",
			0,
			2,
			.05,
			1
		],
		displacement: [
			"Displacement",
			0,
			2,
			.05,
			1
		],
		shading: [
			"Shading",
			0,
			2,
			.05,
			1
		]
	},
	gather: { strength: [
		"Strength",
		0,
		2,
		.1,
		1
	] },
	bloom: { strength: [
		"Strength",
		0,
		2,
		.1,
		.8
	] },
	sparks: {
		strength: [
			"Strength",
			0,
			2,
			.1,
			.8
		],
		count: [
			"Count",
			3,
			18,
			1,
			11
		]
	},
	echo: { strength: [
		"Strength",
		0,
		2,
		.1,
		1
	] },
	printed: {},
	fill: {}
};
function i(e) {
	return Object.fromEntries(Object.entries(r[e.kind]).map(([t, n]) => [t, e.settings[t] ?? n[4]]));
}
function a(e, t = .15) {
	if (e < 0 || e > 1) return 0;
	let n = Math.min(1, e / Math.max(.01, t)), r = Math.max(0, (1 - e) / Math.max(.01, 1 - t));
	return n * n * (3 - 2 * n) * r * r * (3 - 2 * r);
}
var o = 4096, s = class {
	values = new Float64Array(o);
	stamps = new Float64Array(o).fill(-1);
	add(e, t, n = 1, r = .15) {
		let i = Math.ceil(e * 120), s = Math.max(2, Math.ceil(t * 120));
		for (let e = 0; e <= s; e++) {
			let t = i + e, c = t % o;
			this.stamps[c] !== t && (this.stamps[c] = t, this.values[c] = 0), this.values[c] += n * a(e / s, r);
		}
	}
	sample(e) {
		if (e < 0) return 0;
		let t = e * 120, n = Math.floor(t), r = t - n, i = (e) => this.stamps[e % o] === e ? this.values[e % o] : 0;
		return i(n) * (1 - r) + i(n + 1) * r;
	}
};
function c(e, t, n, r = 8, i = 2.4) {
	let a = r + Math.max(0, t) / i, o = Math.max(0, t) / a, s = Math.exp(-a * n);
	e.phase += o * n + (e.speed - o) * (1 - s) / a, e.speed = o + (e.speed - o) * s;
}
//#endregion
//#region work/avatar/expression-model.ts
var l = [
	"idle",
	"typing",
	"submit",
	"working",
	"open",
	"openAfterIdle",
	"complete"
];
function u(e, t) {
	return e.assignments[t] ?? void 0;
}
var d = [
	"mono",
	"aurora",
	"ember",
	"lagoon",
	"oxide",
	"risograph",
	"field",
	"opal",
	"spectrum",
	"slate",
	"moss",
	"plum",
	"ochre",
	"mist",
	"dusk"
], f = [
	"ripple",
	"sparks",
	"echo",
	"bloom"
], p = class e {
	time = 0;
	get phase() {
		return this.flowPhase;
	}
	lightPhase = 0;
	flowPhase = 0;
	visible = !0;
	working = !1;
	momentum = {
		speed: 0,
		phase: 0
	};
	opened = -10;
	settleUntil = 0;
	entrance;
	drives = Object.fromEntries(l.map((e) => [e, {
		motion: new s(),
		color: new s(),
		held: 0,
		colorHeld: 0,
		next: 0
	}]));
	history = Array.from({ length: 4 }, () => new s());
	randomHistory = Array.from({ length: 4 }, () => new s());
	seed = 12345;
	waves = /* @__PURE__ */ new Map();
	phases = /* @__PURE__ */ new Map();
	printOffset = [0, 0];
	inputs = [];
	setup;
	constructor(e) {
		this.setup = e;
	}
	random() {
		return this.seed = Math.imul(this.seed, 1664525) + 1013904223 >>> 0, this.seed / 4294967296;
	}
	emit(e, t = 1) {
		for (let n of e.patterns) {
			let r = f.indexOf(n.kind);
			if (r < 0) continue;
			let a = i(n), o = a.strength, c = n.kind === "ripple" ? Math.max(.1, (a.width ?? .32) * (i(n).travel ?? .65) / 3.14) : e.duration;
			if (this.history[r].add(this.time, c, o * t, .35), n.kind === "ripple") {
				let e = Math.min(15, i(n).travel ?? .65), r = [
					a.originX ?? -.7,
					a.originY ?? -.6,
					e,
					e + c + .1
				], l = [
					a.silhouette ?? 1,
					a.displacement ?? 1,
					a.shading ?? 1,
					1
				], u = JSON.stringify([r, l]), d = this.waves.get(u);
				if (!d) {
					if (this.waves.size >= 8) {
						let e = [...this.waves].sort((e, t) => e[1].last - t[1].last)[0];
						this.waves.delete(e[0]);
					}
					d = {
						curve: new s(),
						profile: r,
						influence: l,
						last: this.time
					}, this.waves.set(u, d);
				}
				d.last = this.time, d.curve.add(this.time, c, o * t, .35);
			}
			if (n.kind === "sparks") for (let e of this.randomHistory) e.add(this.time, c, (this.random() * 2 - 1) * o * t, .35);
		}
	}
	settle(e) {
		e && (this.settleUntil = Math.max(this.settleUntil, this.time + Math.max(4, e.duration, e.color.duration * 4, ...e.patterns.map((e) => (i(e).travel ?? 0) + 1))));
	}
	trigger(e) {
		if (this.inputs.push({
			at: this.time,
			action: e,
			working: this.working,
			visible: this.visible
		}), e === "hide") {
			this.visible = !1;
			return;
		}
		if (e === "complete" && (this.working && this.settle(u(this.setup, "working")), this.working = !1), e === "idle" || e === "working") {
			this.working && this.settle(u(this.setup, "working")), this.working = e === "working";
			let t = u(this.setup, e);
			t?.patterns.some((e) => e.kind === "printed" || e.kind === "fill") && (this.opened = this.time, this.entrance = t);
			return;
		}
		this.visible = !0;
		let t = u(this.setup, e);
		if ((e === "open" || e === "openAfterIdle" || t?.patterns.some((e) => e.kind === "printed" || e.kind === "fill")) && (this.opened = this.time, this.entrance = t), !t) return;
		this.settle(t);
		let n = this.drives[e];
		n.motion.add(this.time, t.duration, 1, t.attack === void 0 ? t.momentum ? .3 : .1 : Math.min(.9, t.attack / t.duration)), n.color.add(this.time, t.color.duration, t.color.strength, Math.min(.9, t.color.attack / t.color.duration)), this.emit(t);
	}
	fork(t) {
		let n = new e(t), r = Math.max(0, this.time - 20), i = this.inputs.filter((e) => e.at >= r);
		n.time = r, n.working = i[0]?.working ?? this.working, n.visible = i[0]?.visible ?? this.visible;
		let a = (e) => {
			for (; n.time < e - 1e-8;) {
				if (!n.visible) {
					n.time = e;
					break;
				}
				n.advance(Math.min(1 / 120, e - n.time));
			}
		};
		for (let e of i) a(e.at), n.trigger(e.action);
		a(this.time), n.time = this.time, n.setPose(this.lightPhase, this.flowPhase), n.printOffset = [...this.printOffset], n.seed = this.seed;
		for (let [e, t] of this.phases) n.phases.has(e) && n.phases.set(e, t);
		return n;
	}
	inspect(e, t = this.time) {
		let n = this.drives[e];
		return {
			input: n.motion.sample(t),
			color: n.color.sample(t),
			ripple: this.history[0].sample(t),
			speed: this.momentum.speed,
			phase: this.momentum.phase
		};
	}
	signals(e) {
		let t = 0, n = 0;
		for (let r of e ? [e] : l) {
			let e = u(this.setup, r);
			if (!e) continue;
			let i = this.drives[r];
			t += i.held + i.motion.sample(this.time), e.color.mode !== "none" && (n += i.color.sample(this.time) + i.colorHeld * e.color.strength);
		}
		return {
			motion: t,
			color: n,
			speed: this.momentum.speed
		};
	}
	colorSignals() {
		return l.flatMap((e) => {
			let t = u(this.setup, e);
			if (!t || t.color.mode === "none") return [];
			let n = t.color.mode === "custom" ? t.color.palette : this.setup.appearance.activityColor;
			if (!n || n === "mono") return [];
			let r = this.drives[e];
			return [{
				palette: n,
				strength: r.color.sample(this.time) + r.colorHeld * t.color.strength
			}];
		});
	}
	needsAnimation() {
		return this.visible ? this.time < this.settleUntil || this.momentum.speed > 1e-4 || l.some((e) => {
			let t = u(this.setup, e);
			if (!t) return !1;
			let n = e === "idle" || e === "working" && this.working ? 1 : 0, r = this.drives[e];
			return Math.abs(r.held - n) > 1e-4 || t.color.mode !== "none" && Math.abs(r.colorHeld - n) > 1e-4 || !!(n && t.patterns.some((e) => {
				let t = i(e);
				return e.kind === "lava" ? t.flowSpeed > 0 || t.lightRotation > 0 : t.strength === 0 ? !1 : f.includes(e.kind) || e.kind === "nudge" || [
					"stir",
					"eddies",
					"drift",
					"sweep",
					"swirl",
					"orbit"
				].includes(e.kind) && t.speed > 0;
			}));
		}) : !1;
	}
	advance(e) {
		if (!this.visible) return;
		let t = Math.max(0, Math.min(.064, e));
		for (this.time += t; this.inputs.length > 1 && this.inputs[1].at < this.time - 20;) this.inputs.shift();
		for (let e of l) {
			let n = u(this.setup, e);
			if (!n) continue;
			let r = this.drives[e], a = e === "idle" || e === "working" && this.working ? 1 : 0;
			r.held += (a - r.held) * (1 - Math.exp(-t * (a ? 10 : 6))), r.colorHeld += (a - r.colorHeld) * (1 - Math.exp(-t * 3 / Math.max(.05, a ? n.color.attack : n.color.duration))), a && r.held > .01 && this.time >= r.next && (this.emit(n, r.held), r.next = this.time + Math.max(.35, ...n.patterns.map((e) => i(e).travel ?? 1.5)));
		}
		let n = this.contributions(), r = this.momentum.phase;
		c(this.momentum, n.push, t, 7, 2.2);
		let a = t * 2.4 * (1 - Math.exp(-n.nudge / 2.4)) + this.momentum.phase - r;
		this.lightPhase += t * n.lightSpeed + a, this.flowPhase += t * n.flowSpeed + a;
		for (let e of n.effects) this.phases.set(e.key, (this.phases.get(e.key) ?? 0) + t * e.speed), e.kind === 6 && (this.printOffset[0] += t * e.amount * e.speed * Math.cos(e.options[2]), this.printOffset[1] += t * e.amount * e.speed * Math.sin(e.options[2]));
		for (let [e, t] of this.waves) this.time - t.last > t.profile[3] && this.waves.delete(e);
	}
	setPose(e, t = e) {
		this.lightPhase = e, this.flowPhase = t;
	}
	contributions() {
		let e = [], t = /* @__PURE__ */ new Map(), n = 0, r = 0, a = 0, o = 0, s = 0, c = !1, f = 0;
		for (let p of l) {
			let l = u(this.setup, p);
			if (!l) continue;
			let m = this.drives[p], h = m.held + m.motion.sample(this.time);
			if (l.color.mode !== "none") {
				let e = d.indexOf(l.color.mode === "custom" ? l.color.palette : this.setup.appearance.activityColor ?? "opal"), n = m.color.sample(this.time) + m.colorHeld * l.color.strength;
				e > 0 && t.set(e, (t.get(e) ?? 0) + n);
			}
			l.patterns.forEach((t, u) => {
				let d = i(t);
				t.kind === "lava" && (c = !0, s += h * d.flowStrength, a += h * .38 * d.lightRotation, o += h * .38 * d.flowSpeed), t.kind === "nudge" && (l.momentum ? n += h * d.strength * 34 : r += h * d.strength * 2), t.kind === "sparks" && (f = Math.max(f, d.count));
				let m = [
					"",
					"pressure",
					"tilt",
					"ruffle",
					"stir",
					"eddies",
					"drift",
					"sweep",
					"swirl",
					"orbit",
					"gather"
				].indexOf(t.kind);
				if (m < 1 || h < 1e-5) return;
				let g = `${p}:${l.id}:${u}:${t.kind}`, _ = d.strength, v = d.speed ?? 0;
				e.push({
					key: g,
					kind: m,
					amount: h * _,
					speed: v,
					coupling: d.distortion ?? 1,
					phase: this.phases.get(g) ?? 0,
					options: [
						d.spread ?? 3,
						d.offset ?? .36,
						t.kind === "sweep" ? d.direction ?? 0 : (d.angle ?? 20) * Math.PI / 180,
						Number(!!l.distortionOnly)
					]
				});
			});
		}
		return {
			effects: e,
			paletteWeights: t,
			push: n,
			nudge: r,
			lightSpeed: a,
			flowSpeed: o,
			flowStrength: c ? s : this.setup.appearance.idleAmount ?? 1.6,
			sparkCount: f
		};
	}
	frame() {
		let { effects: e, paletteWeights: t, flowStrength: n, sparkCount: r } = this.contributions(), a = Math.max(.3, ...Object.values(this.setup.assignments).filter((e) => e !== null).flatMap((e) => e.patterns.filter((e) => e.kind === "ripple").map((e) => i(e).travel ?? .65))), o = [
			Math.min(16, a + .8),
			.6,
			.8,
			.8
		], s = [], c = [];
		for (let e = 0; e < 128; e++) {
			for (let t = 0; t < 4; t++) s.push(this.history[t].sample(this.time - e * o[t] / 127));
			for (let t of this.randomHistory) c.push(t.sample(this.time - e * o[1] / 127));
		}
		let l = Array.from({ length: 1024 }, () => 0), u = Array.from({ length: 32 }, () => 0), d = Array.from({ length: 32 }, () => 0);
		[...this.waves.values()].forEach((e, t) => {
			u.splice(t * 4, 4, ...e.profile), d.splice(t * 4, 4, ...e.influence);
			for (let n = 0; n < 128; n++) l[t * 128 + n] = e.curve.sample(this.time - n * e.profile[3] / 127);
		});
		let f = [...t].filter(([, e]) => e > 1e-4).sort((e, t) => e[0] - t[0]).slice(0, 4).flatMap(([e, t]) => [
			e,
			t,
			0,
			0
		]);
		for (; f.length < 16;) f.push(0);
		let p = this.time - this.opened, m = this.entrance?.patterns.find((e) => e.kind === "printed" || e.kind === "fill"), h = Math.max(0, Math.min(1, this.entrance?.revealStart ?? 0)), g = m ? h + (1 - h) * Math.min(1, p / Math.max(.1, this.entrance.duration)) : 1, _ = e.slice(0, 32), v = _.flatMap((e) => [
			e.kind,
			e.amount,
			e.phase,
			e.coupling
		]), y = _.flatMap((e) => e.options);
		for (; v.length < 128;) v.push(0);
		for (; y.length < 128;) y.push(0);
		return {
			waveTexture: l,
			"u_waveProfiles[0]": u,
			"u_waveInfluences[0]": d,
			u_historySpans: o,
			"u_history[0]": s,
			"u_randomHistory[0]": c,
			"u_accents[0]": f,
			"u_effects[0]": v,
			"u_effectOptions[0]": y,
			u_effectCount: _.length,
			u_phase: this.flowPhase,
			u_lavaPhase: [this.lightPhase, this.flowPhase],
			u_printOffset: [...this.printOffset],
			u_idleAmount: n,
			u_sparkCount: r,
			u_appearance: g,
			u_fillWake: Number(m?.kind === "fill"),
			u_enableOrb: Number(this.visible)
		};
	}
};
Math.PI / 6;
var m = "#version 300 es\nprecision highp float;\nuniform vec2 u_resolution;\nuniform float u_pixelRatio,u_phase,u_breathe,u_texture,u_dark,u_appearance;\nuniform float u_pitch,u_expression,u_enableOrb,u_idleAmount,u_fill,u_contrast,u_matrix;\nuniform float u_color,u_saturation,u_activitySaturation,u_colorStrength;\nuniform float u_volume,u_lightStrength,u_lightAngle,u_toneSteps,u_debugStage,u_print,u_fillWake;\nuniform vec2 u_lavaPhase,u_printOffset;\nuniform vec3 u_paperColor,u_inkColor;\nuniform vec4 u_signal;\nuniform vec4 u_history[128],u_randomHistory[128],u_accents[4];\nuniform vec4 u_historySpans;\nuniform highp sampler2D u_waveCurves;\nuniform vec4 u_waveProfiles[8],u_waveInfluences[8];\nuniform vec4 u_effects[32],u_effectOptions[32];\nuniform float u_effectCount,u_sparkCount;\nuniform vec4 u_recipeDisplacement;\nuniform vec2 u_recipePoke;\nout vec4 fragColor;\nvec2 driftOffset(vec2 v, float transport) {\n  return vec2(sin(v.y*2.4+transport), cos(v.x*2.-transport))*.12;\n}\nvec2 eddyOffset(vec2 v, float phase) {\n  vec2 center=vec2(.24*sin(phase*1.1), .22*cos(phase*.83));\n  vec2 q=v-center;\n  return vec2(-q.y,q.x)*exp(-dot(q,q)*1.7)*.8*sin(phase*2.);\n}\nvec2 agitationField(vec2 v, float phase) {\n  return vec2(sin(v.y*4.+phase*2.7)+.35*sin(v.y*7.-phase*1.9),\n    cos(v.x*3.4-phase*2.1));\n}\nfloat waveAt(float delay,int row,float span){\n  float position=clamp(delay/max(.01,span)*127.,0.,127.);\n  float i=floor(position),y=(float(row)+.5)/8.;\n  float a=texture(u_waveCurves,vec2((i+.5)/128.,y)).r;\n  float b=texture(u_waveCurves,vec2((min(127.,i+1.)+.5)/128.,y)).r;\n  return mix(a,b,fract(position))*step(0.,delay)*step(delay,span);\n}\nvec4 responseAt(float delay){\n  vec4 position=clamp(delay*127./max(vec4(.01),u_historySpans),0.,127.);\n  vec4 outValue=vec4(0.);\n  for(int channel=0;channel<4;channel++){\n    int i=int(floor(position[channel]));\n    outValue[channel]=mix(u_history[i][channel],u_history[min(127,i+1)][channel],fract(position[channel]))*step(0.,delay)*step(delay,u_historySpans[channel]);\n  }\n  return outValue;\n}\nvec4 variationAt(float delay){\n  float position=clamp(delay*127./max(.01,u_historySpans.y),0.,127.);\n  int i=int(floor(position));\n  return mix(u_randomHistory[i],u_randomHistory[min(127,i+1)],fract(position));\n}\nfloat b2(vec2 p){vec2 q=mod(floor(p),2.);return 2.*q.x+3.*q.y-4.*q.x*q.y;}\nfloat rank8(vec2 p){return (16.*b2(p)+4.*b2(floor(p/2.))+b2(floor(p/4.))+.5)/64.;}\nfloat printed(float density,vec2 px){\n  density=clamp(density,0.,1.);\n  if(u_texture<.5){\n    vec2 cell=floor(px/u_pitch);\n    float threshold=u_matrix<3.?(b2(cell)+.5)/4.:u_matrix<5.?(4.*b2(cell)+b2(floor(cell/2.))+.5)/16.:rank8(cell);\n    return step(threshold,density)*step(.001,density);\n  }\n  float spacing=u_pitch*2.;vec2 cell=fract(px/spacing)-.5;float r=sqrt(density)*.7;\n  return (1.-smoothstep(r-.05,r+.05,length(cell)))*step(.012,density);\n}\nvec3 paletteInkSaturation(vec2 p,float phase,float palette,float saturation){\n  float t=.5+.5*sin(p.x*1.8+p.y*.9+phase*.7);\n  float s=.5+.5*cos(p.y*2.-p.x*.8-phase*.5);\n  vec3 a=palette<1.5?vec3(.10,.68,.72):vec3(.95,.29,.15);\n  vec3 b=palette<1.5?vec3(.43,.36,.94):vec3(.96,.63,.22);\n  vec3 c=palette<1.5?vec3(.95,.40,.68):vec3(.71,.23,.54);\n  vec3 col=mix(mix(a,b,t),c,s*.48);\n  if(palette>2.5&&palette<3.5)col=mix(vec3(.08,.42,.62),vec3(.85,.91,.68),smoothstep(.38,.62,t));\n  if(palette>3.5&&palette<4.5)col=mix(vec3(.80,.27,.17),vec3(.12,.65,.59),smoothstep(.38,.62,t));\n  if(palette>4.5){\n    float band=floor(clamp(t*.7+s*.3,0.,.999)*3.);\n    col=band<1.?vec3(.08,.68,.79):band<2.?vec3(.91,.25,.52):vec3(.96,.81,.29);\n  }\n  if(palette>5.5){\n    vec3 x=vec3(.18,.52,.89), y=vec3(.52,.77,.60), z=vec3(.97,.59,.27);\n    if(palette>6.5&&palette<7.5){x=vec3(.52,.68,.90);y=vec3(.76,.61,.83);z=vec3(.93,.80,.61);}\n    if(palette>7.5){x=vec3(.27,.37,.87);y=vec3(.76,.41,.61);z=vec3(.94,.70,.40);}\n    col=t<.5?mix(x,y,smoothstep(0.,.5,t)):mix(y,z,smoothstep(.5,1.,t));\n  }\n  if(palette>8.5&&palette<12.5){\n    col=palette<9.5?vec3(.39,.52,.64):palette<10.5?vec3(.43,.57,.45):palette<11.5?vec3(.62,.46,.59):vec3(.68,.55,.31);\n    col*=.82+t*.25;\n  }\n  if(palette>12.5)col=palette<13.5?mix(vec3(.53,.67,.70),vec3(.77,.73,.66),t):mix(vec3(.54,.51,.68),vec3(.76,.63,.56),t);\n  float luminance=dot(col,vec3(.2126,.7152,.0722));\n  col=mix(vec3(luminance),col,saturation);\n  return mix(col*.7,col+.12,u_dark);\n}\n\nvec3 signalInk(vec2 p,float phase){\n  vec3 base=mix(u_inkColor,paletteInkSaturation(p,phase,u_color,u_saturation),step(.5,u_color)*u_colorStrength);\n  vec3 tint=vec3(0.);float strength=0.;\n  for(int i=0;i<4;i++){tint+=paletteInkSaturation(p,phase,u_accents[i].x,u_activitySaturation)*u_accents[i].y;strength+=u_accents[i].y;}\n  if(strength>.0001){tint/=strength;float l=dot(tint,vec3(.2126,.7152,.0722));tint=mix(tint,tint+(base-vec3(l))*.65,.7);}\n  return mix(base,clamp(tint,0.,1.),min(.8,strength));\n}\nvoid main(){\n  vec2 px=vec2(gl_FragCoord.x,u_resolution.y-gl_FragCoord.y)/u_pixelRatio;\n  vec3 paper=u_paperColor,ink=u_inkColor,color=vec3(0.);float alpha=0.;\n  vec2 lavaPhase=u_lavaPhase;\n  float lightAngle=lavaPhase.x-.7+.24*sin(lavaPhase.x*1.7)+u_lightAngle;\n  vec2 light=vec2(cos(lightAngle),sin(lightAngle));\n  float gather=0.;\n  for(int i=0;i<32;i++){if(float(i)>=u_effectCount)break;if(u_effects[i].x==10.)gather+=u_effects[i].y*u_expression;}\n  float radius=max(1.,u_signal.z)*(1.+sin(u_phase*.7)*.009*u_breathe-.065*min(1.3,gather));\n  vec2 v=(px-u_signal.xy)/radius;\n  float r=length(v);\n  vec3 surface=vec3(0.);\n  vec3 point=normalize(vec3(v,sqrt(max(0.,1.-dot(v,v)))));\n  for(int channel=0;channel<8;channel++){\n    if(u_waveInfluences[channel].w<.5)continue;\n    vec4 profile=u_waveProfiles[channel];\n    vec3 origin=normalize(vec3(profile.xy,.84));\n    float delay=acos(clamp(dot(point,origin),-1.,1.))/3.14159*max(.15,profile.z);\n    surface+=waveAt(delay,channel,profile.w)*.18*u_waveInfluences[channel].xyz;\n  }\n  vec2 posed=v/(1.+surface.x);\n  float distance=length(posed)-1.;\n  float orbMask=(1.-smoothstep(-.025,.025,distance))*u_enableOrb;\n  vec2 displacement=-v*gather*.10;\n  float flow=gather*.14;\n  // Each channel keeps its own parameters and phase through composition.\n  for(int i=0;i<32;i++){\n    if(float(i)>=u_effectCount)break;\n    vec4 effect=u_effects[i],options=u_effectOptions[i];\n    float kind=effect.x,amount=effect.y,t=effect.z,coupling=effect.w;\n    if(kind==1.){\n      vec2 direction=vec2(cos(u_phase*1.3),sin(u_phase*1.3));\n      vec2 q=v-direction*clamp(options.y,0.,.95);\n      displacement+=q*exp(-dot(q,q)*max(.5,options.x))*amount*1.8*u_expression;\n      flow+=dot(v,direction)*amount*.15;\n    }else if(kind==2.){\n      vec2 poke=vec2(.25,.10)*amount;\n      light+=poke*.65*u_expression;flow+=dot(v,poke)*.15;\n    }else if(kind==3.){\n      float kick=min(1.2,amount)*u_expression;\n      vec2 noise=vec2(sin(v.y*4.2+u_phase*2.+.37*5.),cos(v.x*3.8-u_phase*1.7+.37*8.));\n      displacement+=noise*kick*.24;flow+=sin(v.x*3.+v.y*2.+.37*7.)*kick*.12;\n    }else if(kind==4.){\n      vec2 turbulence=agitationField(v,t);\n      displacement+=amount*turbulence*.23*coupling;\n      light+=turbulence*amount*.18*max(0.,coupling-1.);\n      flow+=amount*.15*sin(v.x*3.+v.y*2.-t*3.);\n    }else if(kind==5.){\n      displacement+=eddyOffset(v,t)*amount*coupling;\n      light+=vec2(cos(t*2.3),sin(t*1.7))*amount*.22*coupling;\n    }else if(kind==6.){\n      displacement+=driftOffset(v,t)*amount*coupling;\n      light+=vec2(sin(t),cos(t*.7))*.22*amount*max(0.,coupling-1.);\n      flow+=sin(v.x*3.4+t*3.4)*amount*.10;\n    }else if(kind==7.){\n      float cycle=t*2.3,travel=fract(cycle);\n      vec2 origin=vec2(mod(floor(cycle),2.)<1.?-1.:1.,0.);\n      if(options.z==1.)origin=vec2(1.,0.);\n      else if(options.z==2.)origin=vec2(-1.,0.);\n      else if(options.z==3.)origin=vec2(0.,1.);\n      else if(options.z==4.)origin=vec2(0.,-1.);\n      vec2 normal=normalize(v-origin+vec2(.0001));\n      float front=length(v-origin)-travel*2.9;\n      float crest=exp(-pow(front/.28,2.))*sin(travel*3.14159);\n      displacement+=normal*crest*amount*.32*coupling;\n      light+=normal*crest*amount*.48*max(0.,coupling-1.);\n      flow+=sin(dot(v,normal)*3.-cycle*2.)*crest*amount*.12*max(0.,coupling-1.)+amount*crest*.24;\n    }else if(kind==8.||kind==9.){\n      float angle=atan(v.y,v.x)-t;\n      float crest=kind==8.?sin(angle+.32*sin(t*.73))+.3*sin(2.*angle-t*.31):pow(.5+.5*cos(angle),9.);\n      float ring=exp(-pow((r-.78)/.19,2.))*crest*amount;\n      displacement+=vec2(-v.y,v.x)*ring*.16;flow+=ring*.28*(1.-options.w);\n    }\n  }\n  // Recipe probes expose the same displacement operations at a chosen phase.\n  displacement+=u_recipeDisplacement.x*.24*vec2(sin(u_phase*.9),cos(u_phase*.67));\n  vec2 pq=v-vec2(.2*sin(u_phase),.2*cos(u_phase*.83));\n  displacement+=u_recipeDisplacement.y*vec2(-pq.y,pq.x)*exp(-dot(pq,pq)*1.8)*sin(u_phase*1.6)*.65;\n  displacement+=u_recipeDisplacement.z*.19*vec2(sin(v.y*4.+u_phase*2.7),cos(v.x*3.4-u_phase*2.1));\n  displacement+=vec2(0.,u_recipeDisplacement.w*.30*(1.-smoothstep(.15,1.1,r)));\n  light+=u_recipePoke*.65*u_expression;\n  flow+=dot(v,u_recipePoke)*.15;\n  displacement+=v*surface.y;\n  vec2 sampleV=v-displacement;\n  vec2 samplePx=px-displacement*radius+u_printOffset*radius;\n  float dome=sqrt(max(0.,1.-min(1.,dot(sampleV,sampleV))));\n  flow+=1.4*surface.z;\n  flow+=sin(sampleV.x*2.8+lavaPhase.y*1.4)*cos(sampleV.y*3.1-lavaPhase.y)*.09*u_idleAmount;\n  float tone=.40+.34*dot(sampleV,light)*u_lightStrength+.25*dome*u_volume+flow;\n  tone+=responseAt(length(sampleV-vec2(-.22,.18))*.16).w*.40;\n  tone=clamp((tone-.5)*u_contrast+.5,.015,.985);\n  float fill=u_fillWake>.5?mix(.025,u_fill,smoothstep(0.,1.,u_appearance)):u_fill;\n  tone=fill<.5?tone*fill*2.:mix(tone,1.,(fill-.5)*2.);\n  tone=mix(tone,smoothstep(.10,.90,tone),1.-smoothstep(18.,35.,radius));\n  vec3 orbInk=signalInk(sampleV,u_phase);\n  if(u_toneSteps>1.&&!(u_debugStage>.5&&u_debugStage<5.5)){\n    float level=floor(clamp(tone,0.,.999)*u_toneSteps)/(u_toneSteps-1.);\n    orbInk=mix(paper,orbInk,.30+.70*level);tone=mix(tone,level,.5);\n  }\n  if(u_debugStage>.5&&u_debugStage<5.5){\n    // Intermediate coverage stages for Recipe.\n    float tutorialDome=sqrt(max(0.,1.-min(1.,dot(v,v))));\n    tone=.40;\n    if(u_debugStage>1.5)tone+=.25*tutorialDome*u_volume;\n    if(u_debugStage>2.5)tone+=.34*dot(v,vec2(cos(u_lightAngle),sin(u_lightAngle)))*u_lightStrength;\n    tone=clamp((tone-.5)*u_contrast+.5,.015,.985);\n    tone=u_fill<.5?tone*u_fill*2.:mix(tone,1.,(u_fill-.5)*2.);\n    if(u_debugStage<4.5)orbInk=ink;\n    else orbInk=signalInk(v,u_phase);\n    if(u_debugStage>4.5&&u_toneSteps>1.){float level=floor(clamp(tone,0.,.999)*u_toneSteps)/(u_toneSteps-1.);orbInk=mix(paper,orbInk,.30+.70*level);tone=mix(tone,level,.5);}\n    samplePx=px;\n  }\n  // Isolated recipe components use the same coordinates and phase as the complete field.\n  if(u_debugStage>5.5&&u_debugStage<6.5){tone=dome;orbInk=ink;}\n  if(u_debugStage>6.5&&u_debugStage<7.5){tone=clamp(.5+.5*dot(sampleV,light),0.,1.);orbInk=ink;}\n  if(u_debugStage>7.5&&u_debugStage<8.5){tone=.5+sin(sampleV.x*2.8+lavaPhase.y*1.4)*cos(sampleV.y*3.1-lavaPhase.y)*.09*u_idleAmount;orbInk=ink;}\n  float printAmount=(u_debugStage>.5&&u_debugStage<3.5)||u_debugStage>5.5?0.:u_print;\n  float coverage=orbMask*mix(1.,printed(tone,samplePx),printAmount);\n  vec3 orbColor=mix(paper,orbInk,mix(tone,1.,printAmount));\n  color=orbColor*coverage+color*(1.-coverage);alpha=coverage+alpha*(1.-coverage);\n    float echoDelay=(r-1.)*.9;\n    float echo=responseAt(echoDelay).z*step(1.,r)*exp(-max(0.,r-1.)*6.);\n    float marks=0.;\n    for(int i=0;i<18;i++){\n      if(float(i)>=u_sparkCount)break;\n      float id=float(i),speed=.8+.5*fract(sin(id*27.31+4.)*43758.5);\n      float delay=(r-1.)/(1.7*speed);\n      vec4 response=responseAt(delay);vec4 noise=variationAt(delay)/max(.001,response.y);\n      float jitter=sin(id*13.7+noise.x*8.+noise.y*3.);\n      float angle=id*2.39996+jitter*.7+noise.z*2.;\n      float across=sin(atan(v.y,v.x)-angle)*r;\n      float front=cos(atan(v.y,v.x)-angle);\n      float width=.014+.025*fract(sin(id*17.+noise.w)*351.);\n      float mark=exp(-pow(across/width,2.))*step(.5,front)*response.y;\n      marks=max(marks,mark*step(1.,r)*exp(-max(0.,r-1.)*3.));\n    }\n    marks=printed(max(marks,echo*.65),px)*(1.-orbMask)*u_enableOrb;\n    vec3 tint=signalInk(v,u_phase);\n    color=tint*marks+color*(1.-marks);alpha=marks+alpha*(1.-marks);\n\n  float distanceFromSignal=length((px-u_signal.xy)/(u_signal.z*2.));\n  float appearing=clamp(u_appearance*2.-distanceFromSignal*.75,0.,1.);\n  float visible=step(rank8(floor(px/u_pitch)),appearing);\n  if(u_appearance>=1.)visible=1.;if(u_appearance<=0.)visible=0.;\n  if(u_fillWake>.5)visible=smoothstep(0.,.12,u_appearance);\n  fragColor=vec4(color*visible,alpha*visible);\n}";
//#endregion
//#region work/avatar/preview-frame.ts
function h(e, t, n) {
	let r = t.live && !t.paused, i = r && e.needsAnimation();
	return r && e.advance(n), {
		draw: t.visible && (t.dirty || i),
		keepAlive: r && (e.needsAnimation() || t.visible && t.trackTime)
	};
}
//#endregion
//#region work/avatar/frame-clock.ts
var g = class {
	jobs = /* @__PURE__ */ new Set();
	frame = 0;
	last = 0;
	ticking = !1;
	visible = !0;
	request;
	cancel;
	constructor(e, t) {
		this.request = e, this.cancel = t;
	}
	schedule() {
		!this.frame && !this.ticking && this.visible && [...this.jobs].some((e) => e.active) && (this.frame = this.request((e) => this.tick(e)));
	}
	tick(e) {
		this.frame = 0, this.ticking = !0;
		let t = this.last ? Math.min(64, e - this.last) : 0;
		this.last = e;
		for (let e of this.jobs) e.active &&= e.tick(t);
		this.ticking = !1, this.schedule(), this.frame || (this.last = 0);
	}
	setVisible(e) {
		this.visible = e, this.cancel(this.frame), this.frame = 0, this.last = 0, this.schedule();
	}
	task(e) {
		let t = {
			tick: e,
			active: !1
		};
		return this.jobs.add(t), {
			wake: () => {
				t.active = !0, this.schedule();
			},
			sleep: () => {
				t.active = !1, [...this.jobs].some((e) => e.active) || (this.cancel(this.frame), this.frame = 0, this.last = 0);
			},
			dispose: () => {
				this.jobs.delete(t), [...this.jobs].some((e) => e.active) || (this.cancel(this.frame), this.frame = 0, this.last = 0);
			}
		};
	}
}, _;
function v(e) {
	if (!_) {
		_ = new g((e) => requestAnimationFrame(e), (e) => cancelAnimationFrame(e));
		let e = () => _.setVisible(!document.hidden);
		document.addEventListener("visibilitychange", e), e();
	}
	return _.task(e);
}
//#endregion
//#region work/avatar/preview-colors.ts
function y(e) {
	let t = e ? .9 : .25;
	return [
		t,
		t,
		t
	];
}
//#endregion
//#region work/avatar/appearance-uniforms.ts
function b(e, t) {
	return {
		u_dark: Number(t),
		u_breathe: Number(e.breathe),
		u_texture: Number(e.texture === "halftone"),
		u_pitch: e.pitch,
		u_expression: e.expression,
		u_color: d.indexOf(e.color ?? "mono"),
		u_saturation: e.saturation ?? .45,
		u_activitySaturation: e.activitySaturation ?? .65,
		u_colorStrength: e.colorStrength ?? .5,
		u_fill: e.fill ?? .5,
		u_contrast: e.contrast ?? 1,
		u_matrix: e.matrix ?? 8,
		u_volume: e.volume ?? 1,
		u_lightStrength: e.lightStrength ?? 1,
		u_lightAngle: (e.lightAngle ?? 0) * Math.PI / 180,
		u_toneSteps: e.toneSteps ?? 0,
		u_debugStage: e.debugStage ?? 0,
		u_print: Number(e.effects.indicator),
		u_recipeDisplacement: [
			e.primitiveDrift ?? 0,
			e.primitiveTwist ?? 0,
			e.primitiveNoise ?? 0,
			e.gravity ?? 0
		],
		u_recipePoke: e.inspectPoke ?? [0, 0]
	};
}
//#endregion
//#region work/avatar/preview-renderer.ts
var x = /* @__PURE__ */ new Set(), S, C;
function w() {
	let e = document.createElement("canvas"), t = e.getContext("webgl2", {
		alpha: !0,
		premultipliedAlpha: !0,
		antialias: !1,
		preserveDrawingBuffer: !0
	});
	if (!t) throw Error("WebGL2 unavailable");
	let n = (e, n) => {
		let r = t.createShader(e);
		if (t.shaderSource(r, n), t.compileShader(r), !t.getShaderParameter(r, t.COMPILE_STATUS)) throw Error(t.getShaderInfoLog(r) ?? "Shader compile failed");
		return r;
	}, r = t.createProgram();
	if (t.attachShader(r, n(t.VERTEX_SHADER, "#version 300 es\nin vec2 position;void main(){gl_Position=vec4(position,0.,1.);}")), t.attachShader(r, n(t.FRAGMENT_SHADER, m)), t.linkProgram(r), !t.getProgramParameter(r, t.LINK_STATUS)) throw Error(t.getProgramInfoLog(r) ?? "Shader link failed");
	t.useProgram(r);
	let i = t.createBuffer();
	t.bindBuffer(t.ARRAY_BUFFER, i), t.bufferData(t.ARRAY_BUFFER, new Float32Array([
		-1,
		-1,
		1,
		-1,
		-1,
		1,
		-1,
		1,
		1,
		-1,
		1,
		1
	]), t.STATIC_DRAW);
	let a = t.getAttribLocation(r, "position");
	t.enableVertexAttribArray(a), t.vertexAttribPointer(a, 2, t.FLOAT, !1, 0, 0);
	let o = /* @__PURE__ */ new Map();
	for (let e = 0; e < t.getProgramParameter(r, t.ACTIVE_UNIFORMS); e++) {
		let n = t.getActiveUniform(r, e);
		o.set(n.name, {
			location: t.getUniformLocation(r, n.name),
			type: n.type
		});
	}
	let s = t.createTexture();
	t.activeTexture(t.TEXTURE0), t.bindTexture(t.TEXTURE_2D, s), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_MIN_FILTER, t.NEAREST), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_MAG_FILTER, t.NEAREST), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_WRAP_S, t.CLAMP_TO_EDGE), t.texParameteri(t.TEXTURE_2D, t.TEXTURE_WRAP_T, t.CLAMP_TO_EDGE), t.texImage2D(t.TEXTURE_2D, 0, t.R32F, 128, 8, 0, t.RED, t.FLOAT, null), t.uniform1i(t.getUniformLocation(r, "u_waveCurves"), 0);
	let c = /* @__PURE__ */ new Float32Array(1024);
	function l(e) {
		for (let [n, r] of Object.entries(e)) {
			let e = o.get(n);
			if (!e) continue;
			let { location: i, type: a } = e;
			typeof r == "number" ? t.uniform1f(i, r) : a === t.FLOAT_VEC4 ? t.uniform4fv(i, r) : a === t.FLOAT_VEC3 ? t.uniform3fv(i, r) : a === t.FLOAT_VEC2 ? t.uniform2fv(i, r) : t.uniform1fv(i, r);
		}
	}
	return (n) => {
		let r = n.resolution + 80;
		e.width !== r && (e.width = r, e.height = r, t.viewport(0, 0, r, r));
		let i = n.player.setup.appearance, a = n.player.frame();
		c.set(a.waveTexture), t.texSubImage2D(t.TEXTURE_2D, 0, 0, 0, 128, 8, t.RED, t.FLOAT, c), l({
			...b(i, n.dark),
			...a,
			u_resolution: [r, r],
			u_pixelRatio: 1,
			u_signal: [
				r / 2,
				r / 2,
				n.resolution * .39,
				1
			],
			u_paperColor: n.dark ? [
				.082,
				.086,
				.075
			] : [
				.941,
				.945,
				.921
			],
			u_inkColor: y(n.dark),
			u_colorStrength: i.colorStrength ?? .5
		}), t.clearColor(0, 0, 0, 0), t.clear(t.COLOR_BUFFER_BIT), t.drawArrays(t.TRIANGLES, 0, 6), n.canvas.width !== r && (n.canvas.width = r, n.canvas.height = r);
		let o = n.canvas.getContext("2d");
		o.clearRect(0, 0, r, r), o.drawImage(e, 0, 0), n.canvas.dataset.failed = "false";
	};
}
function T(e) {
	try {
		S ??= w(), S(e), e.onFrame?.(e.player, !0), e.dirty = !1;
	} catch (t) {
		e.canvas.dataset.failed = "true", e.canvas.dataset.error = String(t), console.error(t), e.dirty = !1, e.live = !1;
	}
}
function E(e) {
	let t = e / 1e3, n = !1;
	for (let e of x) {
		let r = h(e.player, e, t);
		if (n ||= r.keepAlive, !e.visible) {
			e.onFrame?.(e.player, !1);
			continue;
		}
		if (!r.draw) {
			e.onFrame?.(e.player, !0);
			continue;
		}
		T(e);
	}
	return n;
}
function D(e, t) {
	let n = {
		canvas: e,
		player: t,
		dark: !0,
		resolution: 96,
		live: !1,
		visible: !0,
		dirty: !0,
		paused: !1,
		trackTime: !1
	};
	x.add(n);
	let r = new IntersectionObserver(([e]) => {
		n.visible = e.isIntersecting, n.dirty = !0, C?.wake();
	});
	return r.observe(e), C ??= v(E), C.wake(), {
		paint: () => T(n),
		update(e) {
			Object.assign(n, e, { dirty: !0 }), C?.wake();
		},
		dispose() {
			r.disconnect(), x.delete(n), x.size || (C?.dispose(), C = void 0);
		}
	};
}
//#endregion
//#region work/live.ts
var O = t();
for (let e of Object.values(O.assignments)) e && (e.color.mode = "none");
var k = O.assignments.idle.patterns.find((e) => e.kind === "lava");
Object.assign(k.settings, {
	flowStrength: 2,
	flowSpeed: 2,
	lightRotation: 1
});
var A = document.querySelector("#live-orb"), j = new p(O);
for (let e = 0; e < 420; e++) j.advance(1 / 60);
var M = D(A, j);
M.update({
	resolution: 640,
	live: !0,
	paused: matchMedia("(prefers-reduced-motion: reduce)").matches,
	dark: !1
}), M.paint(), window.setOrbView = (e) => M.update(e), window.orbReady = A.dataset.failed === "false";
//#endregion
