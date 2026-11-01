//! CosmoForge: An Interactive N-Body Gravitational Laboratory & Spacetime Simulator
//!
//! Fully leverages the system browser with:
//! 1. Real-time N-body orbital physics simulation with collision merging & momentum conservation.
//! 2. Live Spacetime Curvature heatmap rendered via raw RGBA pixel blitting (`put_image_data`).
//! 3. Hardware-accelerated WHATWG Canvas vector graphics (atmospheric glows, trajectory projections, orbital ribbons, debris explosions).
//! 4. Interactive mouse controls (click & drag to aim and sling new planets into orbit).
//! 5. Interactive DOM overlay UI (scenario presets, mass selector, timescale controls, live telemetry dashboard).
//!
//! Run with:
//!     cargo run --manifest-path osbrowser/Cargo.toml --example cosmoforge

use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use osbrowser::browser::{Browser, BrowserDocument, BrowserGuest, BrowserOptions};
use osbrowser::canvas::{Color, Image};
use osbrowser::dom::events::ListenerOptions;

const WIDTH: f32 = 980.0;
const HEIGHT: f32 = 640.0;
const G: f32 = 0.85;
const SOFTENING_SQ: f32 = 180.0;

// Low-res grid for spacetime potential heatmap blitting
const HEATMAP_W: usize = 140;
const HEATMAP_H: usize = 90;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BodyType {
    Star,
    Planet,
    GasGiant,
    Asteroid,
    BlackHole,
}

#[derive(Clone)]
struct Body {
    id: u64,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    mass: f32,
    radius: f32,
    color: Color,
    body_type: BodyType,
    trail: Vec<(f32, f32)>,
}

#[derive(Clone)]
struct Debris {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    color: Color,
    life: f32,
}

struct Simulation {
    bodies: Vec<Body>,
    debris: Vec<Debris>,
    time_scale: f32,
    paused: bool,
    show_trails: bool,
    show_heatmap: bool,
    show_predictions: bool,
    next_body_type: BodyType,
    selected_body_id: Option<u64>,
    drag_start: Option<(f32, f32)>,
    mouse_pos: (f32, f32),
    tick: u64,
    next_id: u64,
    heatmap_pixels: Vec<u8>,
}

impl Simulation {
    fn new() -> Self {
        let mut sim = Self {
            bodies: Vec::new(),
            debris: Vec::new(),
            time_scale: 1.0,
            paused: false,
            show_trails: true,
            show_heatmap: true,
            show_predictions: true,
            next_body_type: BodyType::Planet,
            selected_body_id: None,
            drag_start: None,
            mouse_pos: (WIDTH * 0.5, HEIGHT * 0.5),
            tick: 0,
            next_id: 1,
            heatmap_pixels: vec![0; HEATMAP_W * HEATMAP_H * 4],
        };
        sim.load_solar_system();
        sim
    }

    fn add_body(
        &mut self,
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
        mass: f32,
        radius: f32,
        color: Color,
        body_type: BodyType,
    ) {
        let id = self.next_id;
        self.next_id += 1;
        self.bodies.push(Body {
            id,
            x,
            y,
            vx,
            vy,
            mass,
            radius,
            color,
            body_type,
            trail: Vec::with_capacity(40),
        });
    }

    fn load_solar_system(&mut self) {
        self.bodies.clear();
        self.debris.clear();
        let cx = WIDTH * 0.5;
        let cy = HEIGHT * 0.5;

        // Central Sun
        self.add_body(
            cx, cy, 0.0, 0.0, 3200.0, 24.0,
            Color { r: 253, g: 224, b: 71, a: 255 },
            BodyType::Star,
        );

        // Terrestrial Planet 1 (Inner)
        let r1 = 90.0;
        let v1 = (G * 3200.0 / r1).sqrt();
        self.add_body(
            cx, cy - r1, v1, 0.0, 18.0, 5.0,
            Color { r: 56, g: 189, b: 248, a: 255 },
            BodyType::Planet,
        );

        // Terrestrial Planet 2 (Habitable)
        let r2 = 160.0;
        let v2 = (G * 3200.0 / r2).sqrt();
        self.add_body(
            cx + r2, cy, 0.0, v2, 28.0, 6.5,
            Color { r: 52, g: 211, b: 153, a: 255 },
            BodyType::Planet,
        );

        // Moon orbiting Planet 2
        let m_dist = 18.0;
        let m_v = (G * 28.0 / m_dist).sqrt();
        self.add_body(
            cx + r2 + m_dist, cy, 0.0, v2 + m_v, 2.0, 2.5,
            Color { r: 203, g: 213, b: 225, a: 255 },
            BodyType::Asteroid,
        );

        // Gas Giant with rings
        let r3 = 260.0;
        let v3 = (G * 3200.0 / r3).sqrt();
        self.add_body(
            cx - r3, cy, 0.0, -v3, 210.0, 14.0,
            Color { r: 251, g: 146, b: 60, a: 255 },
            BodyType::GasGiant,
        );

        // Asteroid belt ring
        for i in 0..24 {
            let angle = (i as f32) * (PI * 2.0 / 24.0) + (i as f32 * 0.2);
            let dist = 205.0 + (i % 5) as f32 * 6.0;
            let v = (G * 3200.0 / dist).sqrt();
            let ax = cx + angle.cos() * dist;
            let ay = cy + angle.sin() * dist;
            let avx = -angle.sin() * v;
            let avy = angle.cos() * v;
            self.add_body(
                ax, ay, avx, avy, 2.0, 2.2,
                Color { r: 148, g: 163, b: 184, a: 230 },
                BodyType::Asteroid,
            );
        }
    }

    fn load_binary_system(&mut self) {
        self.bodies.clear();
        self.debris.clear();
        let cx = WIDTH * 0.5;
        let cy = HEIGHT * 0.5;
        let sep = 130.0;
        let star_mass = 1600.0;
        let v = (G * star_mass / (sep * 2.0)).sqrt();

        // Star 1 (Gold)
        self.add_body(
            cx - sep * 0.5, cy, 0.0, -v, star_mass, 19.0,
            Color { r: 251, g: 191, b: 36, a: 255 },
            BodyType::Star,
        );

        // Star 2 (Cyan)
        self.add_body(
            cx + sep * 0.5, cy, 0.0, v, star_mass, 19.0,
            Color { r: 56, g: 189, b: 248, a: 255 },
            BodyType::Star,
        );

        // Planet in circumbinary orbit
        let r_p = 290.0;
        let vp = (G * (star_mass * 2.0) / r_p).sqrt();
        self.add_body(
            cx, cy - r_p, vp, 0.0, 25.0, 6.0,
            Color { r: 244, g: 114, b: 182, a: 255 },
            BodyType::Planet,
        );

        // Chaos comet passing through
        self.add_body(
            cx - 380.0, cy + 180.0, 3.2, -1.8, 4.0, 3.0,
            Color { r: 167, g: 139, b: 250, a: 255 },
            BodyType::Asteroid,
        );
    }

    fn load_figure_eight(&mut self) {
        self.bodies.clear();
        self.debris.clear();
        let cx = WIDTH * 0.5;
        let cy = HEIGHT * 0.5;
        let m = 1200.0;
        let x1 = 150.0;
        let vx = 1.35;
        let vy = 2.45;

        self.add_body(
            cx - x1, cy, vx * 0.5, vy, m, 16.0,
            Color { r: 239, g: 68, b: 68, a: 255 },
            BodyType::Planet,
        );
        self.add_body(
            cx + x1, cy, vx * 0.5, vy, m, 16.0,
            Color { r: 59, g: 130, b: 246, a: 255 },
            BodyType::Planet,
        );
        self.add_body(
            cx, cy, -vx, -vy * 2.0, m, 16.0,
            Color { r: 16, g: 185, b: 129, a: 255 },
            BodyType::Planet,
        );
    }

    fn load_galaxy_collision(&mut self) {
        self.bodies.clear();
        self.debris.clear();

        // Galaxy A (Center Left)
        let cx1 = WIDTH * 0.35;
        let cy1 = HEIGHT * 0.45;
        self.add_body(
            cx1, cy1, 0.7, 0.4, 2500.0, 20.0,
            Color { r: 251, g: 191, b: 36, a: 255 },
            BodyType::Star,
        );
        for i in 0..32 {
            let ang = (i as f32) * 0.35;
            let r = 35.0 + (i as f32) * 3.8;
            let v = (G * 2500.0 / r).sqrt();
            self.add_body(
                cx1 + ang.cos() * r, cy1 + ang.sin() * r,
                0.7 - ang.sin() * v, 0.4 + ang.cos() * v,
                3.0, 2.5,
                Color { r: 253, g: 224, b: 71, a: 220 },
                BodyType::Asteroid,
            );
        }

        // Galaxy B (Center Right)
        let cx2 = WIDTH * 0.65;
        let cy2 = HEIGHT * 0.55;
        self.add_body(
            cx2, cy2, -0.7, -0.4, 2500.0, 20.0,
            Color { r: 56, g: 189, b: 248, a: 255 },
            BodyType::Star,
        );
        for i in 0..32 {
            let ang = (i as f32) * 0.35;
            let r = 35.0 + (i as f32) * 3.8;
            let v = (G * 2500.0 / r).sqrt();
            self.add_body(
                cx2 + ang.cos() * r, cy2 + ang.sin() * r,
                -0.7 + ang.sin() * v, -0.4 - ang.cos() * v,
                3.0, 2.5,
                Color { r: 147, g: 197, b: 253, a: 220 },
                BodyType::Asteroid,
            );
        }
    }

    fn step_physics(&mut self) {
        if self.paused {
            return;
        }

        self.tick += 1;
        let dt = 0.8 * self.time_scale;
        let n = self.bodies.len();

        // 1. Compute pairwise gravitational accelerations
        let mut ax = vec![0.0f32; n];
        let mut ay = vec![0.0f32; n];

        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.bodies[j].x - self.bodies[i].x;
                let dy = self.bodies[j].y - self.bodies[i].y;
                let dist_sq = dx * dx + dy * dy + SOFTENING_SQ;
                let dist = dist_sq.sqrt();
                let f = (G / (dist_sq * dist)) * dt;

                ax[i] += f * self.bodies[j].mass * dx;
                ay[i] += f * self.bodies[j].mass * dy;
                ax[j] -= f * self.bodies[i].mass * dx;
                ay[j] -= f * self.bodies[i].mass * dy;
            }
        }

        // 2. Integrate velocities and positions
        for i in 0..n {
            self.bodies[i].vx += ax[i];
            self.bodies[i].vy += ay[i];
            self.bodies[i].x += self.bodies[i].vx * dt;
            self.bodies[i].y += self.bodies[i].vy * dt;

            // Update trails
            if self.show_trails && self.tick % 3 == 0 {
                let pos = (self.bodies[i].x, self.bodies[i].y);
                self.bodies[i].trail.push(pos);
                if self.bodies[i].trail.len() > 35 {
                    self.bodies[i].trail.remove(0);
                }
            }
        }

        // 3. Handle collisions & inelastic mergers (momentum conservation)
        let mut to_remove = Vec::new();
        for i in 0..self.bodies.len() {
            if to_remove.contains(&i) {
                continue;
            }
            for j in (i + 1)..self.bodies.len() {
                if to_remove.contains(&j) {
                    continue;
                }
                let dx = self.bodies[j].x - self.bodies[i].x;
                let dy = self.bodies[j].y - self.bodies[i].y;
                let r_sum = self.bodies[i].radius + self.bodies[j].radius;
                if dx * dx + dy * dy < r_sum * r_sum {
                    // Decide winner (larger mass absorbs smaller)
                    let (winner, absorbed) = if self.bodies[i].mass >= self.bodies[j].mass {
                        (i, j)
                    } else {
                        (j, i)
                    };

                    let m1 = self.bodies[winner].mass;
                    let m2 = self.bodies[absorbed].mass;
                    let total_m = m1 + m2;

                    // Conservation of momentum
                    self.bodies[winner].vx = (self.bodies[winner].vx * m1 + self.bodies[absorbed].vx * m2) / total_m;
                    self.bodies[winner].vy = (self.bodies[winner].vy * m1 + self.bodies[absorbed].vy * m2) / total_m;
                    self.bodies[winner].mass = total_m;
                    self.bodies[winner].radius = (self.bodies[winner].radius.powi(3) + self.bodies[absorbed].radius.powi(3)).cbrt();

                    // Spawn impact debris particles
                    let impact_x = self.bodies[absorbed].x;
                    let impact_y = self.bodies[absorbed].y;
                    let debris_color = self.bodies[absorbed].color;
                    for k in 0..14 {
                        let ang = (k as f32) * (PI * 2.0 / 14.0);
                        let speed = 2.0 + (k % 4) as f32;
                        self.debris.push(Debris {
                            x: impact_x,
                            y: impact_y,
                            vx: ang.cos() * speed,
                            vy: ang.sin() * speed,
                            color: debris_color,
                            life: 25.0,
                        });
                    }

                    to_remove.push(absorbed);
                }
            }
        }

        // Sort descending so indices stay valid during removal
        to_remove.sort_by(|a, b| b.cmp(a));
        to_remove.dedup();
        for idx in to_remove {
            self.bodies.remove(idx);
        }

        // 4. Update debris particles
        for p in &mut self.debris {
            p.x += p.vx;
            p.y += p.vy;
            p.life -= 1.0;
        }
        self.debris.retain(|p| p.life > 0.0);

        // 5. Update raw spacetime heatmap pixels
        if self.show_heatmap && self.tick % 2 == 0 {
            self.update_heatmap();
        }
    }

    /// Compute gravitational potential field onto low-res pixel grid
    fn update_heatmap(&mut self) {
        let step_x = WIDTH / HEATMAP_W as f32;
        let step_y = HEIGHT / HEATMAP_H as f32;

        for gy in 0..HEATMAP_H {
            let py = (gy as f32 + 0.5) * step_y;
            for gx in 0..HEATMAP_W {
                let px = (gx as f32 + 0.5) * step_x;

                let mut potential = 0.0f32;
                for b in &self.bodies {
                    let dx = b.x - px;
                    let dy = b.y - py;
                    let d = (dx * dx + dy * dy + 400.0).sqrt();
                    potential += b.mass / d;
                }

                // Map potential to color gradient (Deep Space Indigo -> Cyan Warp -> Violet Well)
                let intensity = (potential * 0.045).clamp(0.0, 1.0);
                let idx = (gy * HEATMAP_W + gx) * 4;

                let r = (intensity.powf(2.0) * 160.0) as u8;
                let g = (intensity * 60.0 + (1.0 - intensity) * 8.0) as u8;
                let b = ((intensity * 180.0) + (1.0 - intensity) * 28.0) as u8;

                self.heatmap_pixels[idx] = r;
                self.heatmap_pixels[idx + 1] = g;
                self.heatmap_pixels[idx + 2] = b;
                self.heatmap_pixels[idx + 3] = 255;
            }
        }
    }

    fn spawn_selected_body(&mut self, x: f32, y: f32, vx: f32, vy: f32) {
        let (mass, radius, color) = match self.next_body_type {
            BodyType::Asteroid => (4.0, 3.0, Color { r: 148, g: 163, b: 184, a: 255 }),
            BodyType::Planet => (35.0, 6.5, Color { r: 56, g: 189, b: 248, a: 255 }),
            BodyType::GasGiant => (220.0, 13.0, Color { r: 251, g: 146, b: 60, a: 255 }),
            BodyType::Star => (2800.0, 22.0, Color { r: 253, g: 224, b: 71, a: 255 }),
            BodyType::BlackHole => (8000.0, 16.0, Color { r: 168, g: 85, b: 247, a: 255 }),
        };
        self.add_body(x, y, vx, vy, mass, radius, color, self.next_body_type);
    }
}

struct CosmoApp {
    sim: Arc<Mutex<Simulation>>,
    canvas_node: u32,
    document: BrowserDocument,
}

impl BrowserGuest for CosmoApp {
    fn dispatch_pending(&mut self) -> bool {
        // 1. Process mouse and keyboard interactions
        while let Some(evt) = osbrowser::ui_events::poll() {
            let mut sim = self.sim.lock().unwrap();
            match evt.kind.as_str() {
                "mousemove" => {
                    sim.mouse_pos = (evt.client_x as f32, evt.client_y as f32);
                }
                "mousedown" => {
                    if evt.button == 0 {
                        sim.drag_start = Some((evt.client_x as f32, evt.client_y as f32));
                    }
                }
                "mouseup" => {
                    if evt.button == 0 {
                        if let Some((start_x, start_y)) = sim.drag_start.take() {
                            let end_x = evt.client_x as f32;
                            let end_y = evt.client_y as f32;
                            let vx = (start_x - end_x) * 0.055;
                            let vy = (start_y - end_y) * 0.055;
                            sim.spawn_selected_body(start_x, start_y, vx, vy);
                        }
                    }
                }
                "keydown" => match evt.key.as_str() {
                    " " => sim.paused = !sim.paused,
                    "t" | "T" => sim.show_trails = !sim.show_trails,
                    "h" | "H" => sim.show_heatmap = !sim.show_heatmap,
                    "p" | "P" => sim.show_predictions = !sim.show_predictions,
                    "c" | "C" => { sim.bodies.clear(); sim.debris.clear(); },
                    "1" => sim.load_solar_system(),
                    "2" => sim.load_binary_system(),
                    "3" => sim.load_figure_eight(),
                    "4" => sim.load_galaxy_collision(),
                    _ => {}
                },
                _ => {}
            }
        }

        // 2. Advance physics simulation
        {
            let mut sim = self.sim.lock().unwrap();
            sim.step_physics();
        }

        // 3. Render frame to canvas
        let sim = self.sim.lock().unwrap();
        let mut doc = self.document.write();
        doc.with_canvas_2d(self.canvas_node, |ctx| {
            render_cosmo(ctx, &sim);
        });

        true
    }

    fn next_wakeup(&self) -> Option<Duration> {
        Some(Duration::from_millis(16))
    }
}

fn render_cosmo(ctx: &mut dyn osbrowser::canvas::Canvas, sim: &Simulation) {
    // ── 1. Spacetime Heatmap / Starfield Background ─────────────────
    if sim.show_heatmap {
        let img = Image::from_rgba(HEATMAP_W as u32, HEATMAP_H as u32, sim.heatmap_pixels.clone());
        ctx.draw_image(&img, 0.0, 0.0, WIDTH, HEIGHT);
    } else {
        ctx.set_fill_color(Color { r: 5, g: 8, b: 22, a: 255 });
        ctx.fill_rect(0.0, 0.0, WIDTH, HEIGHT);
    }

    // Distant background star grid
    ctx.set_fill_color(Color { r: 255, g: 255, b: 255, a: 110 });
    for i in 0..40 {
        let sx = ((i * 137) as f32) % WIDTH;
        let sy = ((i * 229) as f32) % HEIGHT;
        ctx.fill_rect(sx, sy, 1.5, 1.5);
    }

    // ── 2. Orbital Trajectory Prediction Lines ──────────────────────
    if sim.show_predictions && !sim.bodies.is_empty() {
        let mut sim_bodies: Vec<(f32, f32, f32, f32, f32)> = sim
            .bodies
            .iter()
            .map(|b| (b.x, b.y, b.vx, b.vy, b.mass))
            .collect();

        let dt = 1.2;
        let steps = 40;
        let n = sim_bodies.len();

        for _ in 0..steps {
            let mut ax = vec![0.0f32; n];
            let mut ay = vec![0.0f32; n];
            for i in 0..n {
                for j in (i + 1)..n {
                    let dx = sim_bodies[j].0 - sim_bodies[i].0;
                    let dy = sim_bodies[j].1 - sim_bodies[i].1;
                    let dist_sq = dx * dx + dy * dy + SOFTENING_SQ;
                    let f = (G / (dist_sq * dist_sq.sqrt())) * dt;
                    ax[i] += f * sim_bodies[j].4 * dx;
                    ay[i] += f * sim_bodies[j].4 * dy;
                    ax[j] -= f * sim_bodies[i].4 * dx;
                    ay[j] -= f * sim_bodies[i].4 * dy;
                }
            }
            for i in 0..n {
                sim_bodies[i].2 += ax[i];
                sim_bodies[i].3 += ay[i];
                let prev_x = sim_bodies[i].0;
                let prev_y = sim_bodies[i].1;
                sim_bodies[i].0 += sim_bodies[i].2 * dt;
                sim_bodies[i].1 += sim_bodies[i].3 * dt;

                ctx.set_stroke_color(Color { r: 148, g: 163, b: 184, a: 60 });
                ctx.set_line_width(1.0);
                ctx.begin_path();
                ctx.move_to(prev_x, prev_y);
                ctx.line_to(sim_bodies[i].0, sim_bodies[i].1);
                ctx.stroke();
            }
        }
    }

    // ── 3. Fading Orbit Ribbon Trails ───────────────────────────────
    if sim.show_trails {
        for b in &sim.bodies {
            if b.trail.len() > 1 {
                ctx.set_stroke_color(Color {
                    r: b.color.r,
                    g: b.color.g,
                    b: b.color.b,
                    a: 140,
                });
                ctx.set_line_width(1.5);
                ctx.begin_path();
                ctx.move_to(b.trail[0].0, b.trail[0].1);
                for &(tx, ty) in &b.trail[1..] {
                    ctx.line_to(tx, ty);
                }
                ctx.line_to(b.x, b.y);
                ctx.stroke();
            }
        }
    }

    // ── 4. Impact Debris Particles ──────────────────────────────────
    for p in &sim.debris {
        ctx.set_fill_color(p.color);
        ctx.begin_path();
        ctx.arc(p.x, p.y, (p.life * 0.12).max(1.0), 0.0, PI * 2.0, false);
        ctx.fill();
    }

    // ── 5. Celestial Bodies & Glow Halos ─────────────────────────────
    for b in &sim.bodies {
        // Multi-tier Atmospheric Glow Halos
        match b.body_type {
            BodyType::Star => {
                let pulse = (sim.tick as f32 * 0.08).sin() * 2.5;
                ctx.set_fill_color(Color { r: 254, g: 240, b: 138, a: 35 });
                ctx.begin_path();
                ctx.arc(b.x, b.y, b.radius * 2.8 + pulse, 0.0, PI * 2.0, false);
                ctx.fill();

                ctx.set_fill_color(Color { r: 253, g: 224, b: 71, a: 80 });
                ctx.begin_path();
                ctx.arc(b.x, b.y, b.radius * 1.6, 0.0, PI * 2.0, false);
                ctx.fill();
            }
            BodyType::BlackHole => {
                // Purple accretion glow ring
                ctx.set_stroke_color(Color { r: 168, g: 85, b: 247, a: 180 });
                ctx.set_line_width(3.0);
                ctx.begin_path();
                ctx.arc(b.x, b.y, b.radius * 1.8, 0.0, PI * 2.0, false);
                ctx.stroke();
            }
            BodyType::GasGiant => {
                // Planetary ring system
                ctx.set_stroke_color(Color { r: 253, g: 186, b: 116, a: 150 });
                ctx.set_line_width(2.0);
                ctx.begin_path();
                ctx.arc(b.x, b.y, b.radius * 1.9, 0.0, PI * 2.0, false);
                ctx.stroke();
            }
            _ => {}
        }

        // Solid Body Sphere
        ctx.set_fill_color(b.color);
        ctx.begin_path();
        ctx.arc(b.x, b.y, b.radius, 0.0, PI * 2.0, false);
        ctx.fill();

        // Edge stroke highlight
        ctx.set_stroke_color(Color { r: 255, g: 255, b: 255, a: 180 });
        ctx.set_line_width(1.0);
        ctx.stroke();
    }

    // ── 6. Mouse Launch Sling Vector Arrow ───────────────────────────
    if let Some((sx, sy)) = sim.drag_start {
        let (mx, my) = sim.mouse_pos;

        // Slingshot trajectory line
        ctx.set_stroke_color(Color { r: 52, g: 211, b: 153, a: 240 });
        ctx.set_line_width(2.5);
        ctx.begin_path();
        ctx.move_to(sx, sy);
        ctx.line_to(mx, my);
        ctx.stroke();

        // Launch anchor circle
        ctx.set_fill_color(Color { r: 52, g: 211, b: 153, a: 180 });
        ctx.begin_path();
        ctx.arc(sx, sy, 6.0, 0.0, PI * 2.0, false);
        ctx.fill();

        // Target preview icon
        ctx.set_stroke_color(Color { r: 52, g: 211, b: 153, a: 255 });
        ctx.begin_path();
        ctx.arc(mx, my, 4.0, 0.0, PI * 2.0, false);
        ctx.stroke();
    }

    // ── 7. HUD Status Overlay ────────────────────────────────────────
    ctx.set_fill_color(Color { r: 2, g: 6, b: 23, a: 180 });
    ctx.fill_rect(12.0, 12.0, 310.0, 48.0);
    ctx.set_stroke_color(Color { r: 255, g: 255, b: 255, a: 30 });
    ctx.set_line_width(1.0);
    ctx.stroke_rect(12.0, 12.0, 310.0, 48.0);

    ctx.set_font_css("bold 13px 'Courier New', monospace");
    ctx.set_fill_color(Color { r: 56, g: 189, b: 248, a: 255 });
    ctx.fill_text(
        &format!("BODIES: {:02}   HEATMAP: {}", sim.bodies.len(), if sim.show_heatmap { "ON" } else { "OFF" }),
        24.0, 32.0,
    );

    ctx.set_fill_color(Color { r: 148, g: 163, b: 184, a: 255 });
    ctx.fill_text(
        &format!("SPEED: {:.1}x   STATUS: {}", sim.time_scale, if sim.paused { "PAUSED" } else { "RUNNING" }),
        24.0, 49.0,
    );
}

fn main() {
    let html = r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>CosmoForge - Spacetime & Orbital Gravity Laboratory</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; user-select: none; }
    body {
      background: #020617;
      font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", Roboto, sans-serif;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 16px;
      color: #f8fafc;
      overflow: hidden;
    }
    .wrapper {
      position: relative;
      display: flex;
      flex-direction: column;
      align-items: center;
    }
    .canvas-container {
      position: relative;
      box-shadow: 0 25px 60px rgba(0, 0, 0, 0.9), 0 0 0 1px rgba(255, 255, 255, 0.1);
      border-radius: 16px;
      overflow: hidden;
      background: #020617;
    }
    canvas {
      display: block;
      cursor: crosshair;
    }
    /* Floating Glassmorphic Control Strip */
    .controls-panel {
      position: absolute;
      top: 14px;
      right: 14px;
      display: flex;
      gap: 8px;
      background: rgba(15, 23, 42, 0.85);
      border: 1px solid rgba(255, 255, 255, 0.12);
      backdrop-filter: blur(16px);
      padding: 8px 12px;
      border-radius: 12px;
      box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
    }
    .btn {
      background: rgba(30, 41, 59, 0.8);
      color: #cbd5e1;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 8px;
      padding: 6px 12px;
      font-size: 12px;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.15s ease;
      outline: none;
    }
    .btn:hover { background: #334155; color: #f8fafc; }
    .btn.active {
      background: #38bdf8;
      color: #030712;
      font-weight: 600;
      border-color: #38bdf8;
    }

    /* Bottom Presets Bar */
    .presets-bar {
      margin-top: 14px;
      display: flex;
      gap: 10px;
      align-items: center;
      background: rgba(15, 23, 42, 0.7);
      border: 1px solid rgba(255, 255, 255, 0.08);
      padding: 8px 16px;
      border-radius: 12px;
      font-size: 13px;
    }
    .preset-label { color: #94a3b8; font-weight: 500; margin-right: 4px; }
  </style>
</head>
<body>
  <div class="wrapper">
    <div class="canvas-container">
      <canvas id="cosmo_canvas" width="980" height="640"></canvas>

      <!-- Overlay Quick Controls -->
      <div class="controls-panel">
        <button id="btn_toggle_heatmap" class="btn active">Spacetime Heatmap</button>
        <button id="btn_toggle_trails" class="btn active">Orbit Trails</button>
        <button id="btn_toggle_predict" class="btn active">Predictions</button>
        <button id="btn_toggle_pause" class="btn">Pause (Space)</button>
        <button id="btn_clear" class="btn">Clear</button>
      </div>
    </div>

    <!-- Presets Bar -->
    <div class="presets-bar">
      <span class="preset-label">Scenarios:</span>
      <button id="preset_solar" class="btn active">Solar System</button>
      <button id="preset_binary" class="btn">Binary Stars</button>
      <button id="preset_figure8" class="btn">3-Body Chaos</button>
      <button id="preset_galaxy" class="btn">Galaxy Collision</button>
      <span class="preset-label" style="margin-left: 12px;">Spawn Tool:</span>
      <button id="spawn_planet" class="btn active">Planet</button>
      <button id="spawn_giant" class="btn">Gas Giant</button>
      <button id="spawn_star" class="btn">Star</button>
      <button id="spawn_blackhole" class="btn">Black Hole</button>
    </div>
  </div>
</body>
</html>"#;

    let document = BrowserDocument::new(osbrowser::load_html(html, 1020.0));
    let sim = Arc::new(Mutex::new(Simulation::new()));

    let canvas_node = {
        let mut doc = document.write();
        let c_node = doc.query_selector("#cosmo_canvas").expect("canvas exists");

        // Helper macro for attaching click event listeners
        let attach = |doc: &mut osbrowser::types::Document, id: &str, f: Box<dyn Fn(&mut osbrowser::types::Document) + Send + Sync>| {
            if let Some(node) = doc.query_selector(id) {
                doc.add_event_listener(
                    node,
                    "click",
                    Box::new(move |_evt, doc| {
                        f(doc);
                    }),
                    ListenerOptions::default(),
                );
            }
        };

        // Attach Scenario Preset buttons
        let s1 = Arc::clone(&sim);
        attach(&mut doc, "#preset_solar", Box::new(move |_doc| { s1.lock().unwrap().load_solar_system(); }));

        let s2 = Arc::clone(&sim);
        attach(&mut doc, "#preset_binary", Box::new(move |_doc| { s2.lock().unwrap().load_binary_system(); }));

        let s3 = Arc::clone(&sim);
        attach(&mut doc, "#preset_figure8", Box::new(move |_doc| { s3.lock().unwrap().load_figure_eight(); }));

        let s4 = Arc::clone(&sim);
        attach(&mut doc, "#preset_galaxy", Box::new(move |_doc| { s4.lock().unwrap().load_galaxy_collision(); }));

        // Attach Feature Toggles
        let st_heat = Arc::clone(&sim);
        attach(&mut doc, "#btn_toggle_heatmap", Box::new(move |_doc| {
            let mut s = st_heat.lock().unwrap();
            s.show_heatmap = !s.show_heatmap;
        }));

        let st_trails = Arc::clone(&sim);
        attach(&mut doc, "#btn_toggle_trails", Box::new(move |_doc| {
            let mut s = st_trails.lock().unwrap();
            s.show_trails = !s.show_trails;
        }));

        let st_predict = Arc::clone(&sim);
        attach(&mut doc, "#btn_toggle_predict", Box::new(move |_doc| {
            let mut s = st_predict.lock().unwrap();
            s.show_predictions = !s.show_predictions;
        }));

        let st_pause = Arc::clone(&sim);
        attach(&mut doc, "#btn_toggle_pause", Box::new(move |_doc| {
            let mut s = st_pause.lock().unwrap();
            s.paused = !s.paused;
        }));

        let st_clear = Arc::clone(&sim);
        attach(&mut doc, "#btn_clear", Box::new(move |_doc| {
            let mut s = st_clear.lock().unwrap();
            s.bodies.clear();
            s.debris.clear();
        }));

        // Attach Spawn Tool buttons
        let sp_planet = Arc::clone(&sim);
        attach(&mut doc, "#spawn_planet", Box::new(move |_doc| { sp_planet.lock().unwrap().next_body_type = BodyType::Planet; }));

        let sp_giant = Arc::clone(&sim);
        attach(&mut doc, "#spawn_giant", Box::new(move |_doc| { sp_giant.lock().unwrap().next_body_type = BodyType::GasGiant; }));

        let sp_star = Arc::clone(&sim);
        attach(&mut doc, "#spawn_star", Box::new(move |_doc| { sp_star.lock().unwrap().next_body_type = BodyType::Star; }));

        let sp_bh = Arc::clone(&sim);
        attach(&mut doc, "#spawn_blackhole", Box::new(move |_doc| { sp_bh.lock().unwrap().next_body_type = BodyType::BlackHole; }));

        c_node
    };

    let mut browser = Browser::open(BrowserOptions {
        title: "CosmoForge - Spacetime Laboratory".into(),
        width: 1060.0,
        height: 760.0,
        ..Default::default()
    })
    .expect("open browser");

    browser.attach_document(document.clone());
    browser.set_guest(CosmoApp {
        sim,
        canvas_node,
        document,
    });

    println!("===========================================================");
    println!("  CosmoForge Spacetime Laboratory running in browser tab!  ");
    println!("  • Drag mouse on canvas to aim & launch celestial bodies   ");
    println!("  • Presets: Solar System, Binary Stars, 3-Body, Galaxy    ");
    println!("  • Space: Pause  •  T: Trails  •  H: Heatmap  •  C: Clear ");
    println!("===========================================================");

    browser.run().expect("browser loop ended");
}
