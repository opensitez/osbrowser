//! Classic Scrolling Platformer Game running natively in the system browser
//! using osbrowser's WHATWG Canvas bridge.
//!
//! Run with:
//!     cargo run --manifest-path osbrowser/Cargo.toml --example platformer

use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use osbrowser::browser::{Browser, BrowserDocument, BrowserGuest, BrowserOptions};
use osbrowser::canvas::Color;

const SCREEN_W: f32 = 800.0;
const SCREEN_H: f32 = 500.0;
const WORLD_W: f32 = 3600.0;
const GRAVITY: f32 = 0.55;
const JUMP_FORCE: f32 = -12.5;
const RUN_SPEED: f32 = 4.8;
const ACCEL: f32 = 0.65;
const FRICTION: f32 = 0.82;

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
}

struct Coin {
    x: f32,
    y: f32,
    collected: bool,
}

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    color: Color,
    life: f32,
}

struct MovingPlatform {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    min_x: f32,
    max_x: f32,
    speed: f32,
}

#[derive(Default)]
struct Input {
    left: bool,
    right: bool,
    jump: bool,
    jump_pressed: bool,
}

struct Player {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    w: f32,
    h: f32,
    on_ground: bool,
    facing_right: bool,
    anim_timer: f32,
}

struct GameState {
    player: Player,
    input: Input,
    camera_x: f32,
    score: u32,
    coins_collected: u32,
    won: bool,
    platforms: Vec<Rect>,
    moving_platforms: Vec<MovingPlatform>,
    coins: Vec<Coin>,
    particles: Vec<Particle>,
    tick: u64,
}

impl GameState {
    fn new() -> Self {
        let mut platforms = Vec::new();

        // Ground segments with occasional gaps
        platforms.push(Rect { x: 0.0, y: 440.0, w: 900.0, h: 60.0 });
        platforms.push(Rect { x: 980.0, y: 440.0, w: 700.0, h: 60.0 });
        platforms.push(Rect { x: 1760.0, y: 440.0, w: 800.0, h: 60.0 });
        platforms.push(Rect { x: 2640.0, y: 440.0, w: 960.0, h: 60.0 });

        // Floating ledges & obstacles
        platforms.push(Rect { x: 250.0, y: 350.0, w: 120.0, h: 22.0 });
        platforms.push(Rect { x: 420.0, y: 280.0, w: 140.0, h: 22.0 });
        platforms.push(Rect { x: 620.0, y: 220.0, w: 120.0, h: 22.0 });
        platforms.push(Rect { x: 800.0, y: 320.0, w: 100.0, h: 22.0 });

        // Mid section platforms
        platforms.push(Rect { x: 1080.0, y: 340.0, w: 130.0, h: 22.0 });
        platforms.push(Rect { x: 1280.0, y: 260.0, w: 120.0, h: 22.0 });
        platforms.push(Rect { x: 1470.0, y: 190.0, w: 140.0, h: 22.0 });
        platforms.push(Rect { x: 1680.0, y: 290.0, w: 110.0, h: 22.0 });

        // High towers & staircases
        platforms.push(Rect { x: 1900.0, y: 370.0, w: 80.0, h: 22.0 });
        platforms.push(Rect { x: 2020.0, y: 310.0, w: 80.0, h: 22.0 });
        platforms.push(Rect { x: 2140.0, y: 250.0, w: 80.0, h: 22.0 });
        platforms.push(Rect { x: 2260.0, y: 190.0, w: 100.0, h: 22.0 });
        platforms.push(Rect { x: 2420.0, y: 270.0, w: 140.0, h: 22.0 });

        // Final approach to goal
        platforms.push(Rect { x: 2750.0, y: 360.0, w: 100.0, h: 22.0 });
        platforms.push(Rect { x: 2920.0, y: 290.0, w: 120.0, h: 22.0 });
        platforms.push(Rect { x: 3100.0, y: 230.0, w: 140.0, h: 22.0 });

        // Moving platforms across chasms
        let moving_platforms = vec![
            MovingPlatform {
                x: 900.0,
                y: 390.0,
                w: 80.0,
                h: 18.0,
                min_x: 880.0,
                max_x: 990.0,
                speed: 1.6,
            },
            MovingPlatform {
                x: 1670.0,
                y: 380.0,
                w: 80.0,
                h: 18.0,
                min_x: 1660.0,
                max_x: 1780.0,
                speed: 2.0,
            },
            MovingPlatform {
                x: 2550.0,
                y: 380.0,
                w: 85.0,
                h: 18.0,
                min_x: 2530.0,
                max_x: 2660.0,
                speed: 2.2,
            },
        ];

        // Coins scattered throughout
        let mut coins = Vec::new();
        let coin_locs = [
            (280.0, 310.0), (320.0, 310.0),
            (450.0, 240.0), (490.0, 240.0), (530.0, 240.0),
            (650.0, 180.0), (690.0, 180.0),
            (1110.0, 300.0), (1150.0, 300.0),
            (1310.0, 220.0), (1350.0, 220.0),
            (1500.0, 150.0), (1540.0, 150.0), (1580.0, 150.0),
            (1930.0, 330.0), (2050.0, 270.0), (2170.0, 210.0), (2290.0, 150.0),
            (2800.0, 320.0), (2970.0, 250.0), (3150.0, 190.0),
        ];
        for (cx, cy) in coin_locs {
            coins.push(Coin { x: cx, y: cy, collected: false });
        }

        Self {
            player: Player {
                x: 80.0,
                y: 380.0,
                vx: 0.0,
                vy: 0.0,
                w: 26.0,
                h: 36.0,
                on_ground: false,
                facing_right: true,
                anim_timer: 0.0,
            },
            input: Input::default(),
            camera_x: 0.0,
            score: 0,
            coins_collected: 0,
            won: false,
            platforms,
            moving_platforms,
            coins,
            particles: Vec::new(),
            tick: 0,
        }
    }

    fn update(&mut self) {
        self.tick += 1;

        // Update moving platforms
        for p in &mut self.moving_platforms {
            p.x += p.speed;
            if p.x > p.max_x || p.x < p.min_x {
                p.speed = -p.speed;
            }
        }

        // Horizontal movement
        if self.input.left {
            self.player.vx -= ACCEL;
            self.player.facing_right = false;
        } else if self.input.right {
            self.player.vx += ACCEL;
            self.player.facing_right = true;
        } else {
            self.player.vx *= FRICTION;
            if self.player.vx.abs() < 0.1 {
                self.player.vx = 0.0;
            }
        }
        self.player.vx = self.player.vx.clamp(-RUN_SPEED, RUN_SPEED);

        // Jump
        if self.input.jump_pressed && self.player.on_ground {
            self.player.vy = JUMP_FORCE;
            self.player.on_ground = false;

            // Jump puff particles
            for _ in 0..6 {
                let angle = (rand_f32(self.tick) * PI) - PI;
                self.particles.push(Particle {
                    x: self.player.x + self.player.w * 0.5,
                    y: self.player.y + self.player.h,
                    vx: angle.cos() * 2.0,
                    vy: angle.sin().abs() * -1.5,
                    color: Color { r: 240, g: 240, b: 240, a: 220 },
                    life: 18.0,
                });
            }
        }
        self.input.jump_pressed = false;

        // Apply gravity
        self.player.vy += GRAVITY;
        if self.player.vy > 12.0 {
            self.player.vy = 12.0;
        }

        // Move horizontally & collide
        self.player.x += self.player.vx;
        let mut player_box = Rect {
            x: self.player.x,
            y: self.player.y,
            w: self.player.w,
            h: self.player.h,
        };

        for plat in &self.platforms {
            if player_box.intersects(plat) {
                if self.player.vx > 0.0 {
                    self.player.x = plat.x - self.player.w;
                } else if self.player.vx < 0.0 {
                    self.player.x = plat.x + plat.w;
                }
                self.player.vx = 0.0;
                player_box.x = self.player.x;
            }
        }

        // Move vertically & collide
        self.player.y += self.player.vy;
        player_box.y = self.player.y;
        self.player.on_ground = false;

        // Check static platforms
        for plat in &self.platforms {
            if player_box.intersects(plat) {
                if self.player.vy > 0.0 {
                    self.player.y = plat.y - self.player.h;
                    self.player.vy = 0.0;
                    self.player.on_ground = true;
                } else if self.player.vy < 0.0 {
                    self.player.y = plat.y + plat.h;
                    self.player.vy = 0.0;
                }
                player_box.y = self.player.y;
            }
        }

        // Check moving platforms (one-way top landing)
        for mp in &self.moving_platforms {
            let plat = Rect { x: mp.x, y: mp.y, w: mp.w, h: mp.h };
            if player_box.intersects(&plat) && self.player.vy >= 0.0 && self.player.y + self.player.h - self.player.vy <= mp.y + 6.0 {
                self.player.y = mp.y - self.player.h;
                self.player.vy = 0.0;
                self.player.x += mp.speed;
                self.player.on_ground = true;
                player_box.y = self.player.y;
            }
        }

        // Fall out of world reset
        if self.player.y > SCREEN_H + 100.0 {
            self.player.x = (self.camera_x + 50.0).max(60.0);
            self.player.y = 100.0;
            self.player.vy = 0.0;
            self.player.vx = 0.0;
        }

        // Animation timer
        if self.player.vx.abs() > 0.5 && self.player.on_ground {
            self.player.anim_timer += 0.25;
        } else {
            self.player.anim_timer = 0.0;
        }

        // Coin collection
        let p_rect = Rect {
            x: self.player.x,
            y: self.player.y,
            w: self.player.w,
            h: self.player.h,
        };
        for coin in &mut self.coins {
            if !coin.collected {
                let c_rect = Rect { x: coin.x - 12.0, y: coin.y - 12.0, w: 24.0, h: 24.0 };
                if p_rect.intersects(&c_rect) {
                    coin.collected = true;
                    self.score += 100;
                    self.coins_collected += 1;

                    // Sparkle particles
                    for i in 0..10 {
                        let ang = (i as f32) * (PI * 2.0 / 10.0);
                        self.particles.push(Particle {
                            x: coin.x,
                            y: coin.y,
                            vx: ang.cos() * 3.5,
                            vy: ang.sin() * 3.5,
                            color: Color { r: 251, g: 191, b: 36, a: 255 },
                            life: 22.0,
                        });
                    }
                }
            }
        }

        // Goal flagpole check at x = 3360
        if self.player.x >= 3360.0 && !self.won {
            self.won = true;
            self.score += 2000;
            for i in 0..30 {
                let ang = (i as f32) * (PI * 2.0 / 30.0);
                self.particles.push(Particle {
                    x: 3380.0,
                    y: 260.0,
                    vx: ang.cos() * 5.0,
                    vy: ang.sin() * 5.0,
                    color: Color { r: 52, g: 211, b: 153, a: 255 },
                    life: 45.0,
                });
            }
        }

        // Update particles
        for p in &mut self.particles {
            p.x += p.vx;
            p.y += p.vy;
            p.vy += 0.12;
            p.life -= 1.0;
        }
        self.particles.retain(|p| p.life > 0.0);

        // Smooth camera follow
        let target_camera = self.player.x - 300.0;
        self.camera_x += (target_camera - self.camera_x) * 0.12;
        self.camera_x = self.camera_x.clamp(0.0, WORLD_W - SCREEN_W);
    }
}

fn rand_f32(seed: u64) -> f32 {
    let mut x = seed.wrapping_mul(0xd8163841);
    x ^= x >> 16;
    (x as u32 % 1000) as f32 / 1000.0
}

struct PlatformerGame {
    state: Arc<Mutex<GameState>>,
    canvas_node: u32,
    document: BrowserDocument,
}

impl BrowserGuest for PlatformerGame {
    fn dispatch_pending(&mut self) -> bool {
        // 1. Poll browser UI events for player controls
        while let Some(event) = osbrowser::ui_events::poll() {
            let mut state = self.state.lock().unwrap();
            match event.kind.as_str() {
                "keydown" => match event.key.as_str() {
                    "ArrowLeft" | "a" | "A" => state.input.left = true,
                    "ArrowRight" | "d" | "D" => state.input.right = true,
                    "ArrowUp" | "w" | "W" | " " => {
                        state.input.jump = true;
                        state.input.jump_pressed = true;
                    }
                    "r" | "R" => *state = GameState::new(),
                    _ => {}
                },
                "keyup" => match event.key.as_str() {
                    "ArrowLeft" | "a" | "A" => state.input.left = false,
                    "ArrowRight" | "d" | "D" => state.input.right = false,
                    "ArrowUp" | "w" | "W" | " " => state.input.jump = false,
                    _ => {}
                },
                _ => {}
            }
        }

        // 2. Step game physics & logic
        {
            let mut state = self.state.lock().unwrap();
            state.update();
        }

        // 3. Render frame to the browser canvas
        let state = self.state.lock().unwrap();
        let mut doc = self.document.write();
        doc.with_canvas_2d(self.canvas_node, |ctx| {
            render_frame(ctx, &state);
        });

        true
    }

    fn next_wakeup(&self) -> Option<Duration> {
        Some(Duration::from_millis(16))
    }
}

fn render_frame(ctx: &mut dyn osbrowser::canvas::Canvas, state: &GameState) {
    let cam = state.camera_x;

    // ── 1. Sky & Atmosphere ──────────────────────────────────────────
    ctx.set_fill_color(Color { r: 14, g: 165, b: 233, a: 255 }); // Bright sky
    ctx.fill_rect(0.0, 0.0, SCREEN_W, SCREEN_H);

    // Sun
    let sun_x = 700.0 - cam * 0.04;
    ctx.set_fill_color(Color { r: 254, g: 240, b: 138, a: 240 });
    ctx.begin_path();
    ctx.arc(sun_x, 80.0, 42.0, 0.0, PI * 2.0, false);
    ctx.fill();

    // ── 2. Distant Parallax Mountains (0.15x speed) ──────────────────
    ctx.set_fill_color(Color { r: 56, g: 189, b: 248, a: 160 });
    ctx.begin_path();
    let mtn_offset = cam * 0.15;
    ctx.move_to(0.0, SCREEN_H);
    for i in 0..12 {
        let mx = (i as f32) * 180.0 - (mtn_offset % 180.0);
        let my = 260.0 + if i % 2 == 0 { -70.0 } else { -30.0 };
        ctx.line_to(mx, my);
        ctx.line_to(mx + 90.0, 360.0);
    }
    ctx.line_to(SCREEN_W, SCREEN_H);
    ctx.close_path();
    ctx.fill();

    // ── 3. Midground Hills & Trees (0.35x speed) ─────────────────────
    ctx.set_fill_color(Color { r: 16, g: 185, b: 129, a: 180 });
    ctx.begin_path();
    let hill_offset = cam * 0.35;
    ctx.move_to(0.0, SCREEN_H);
    for i in 0..10 {
        let hx = (i as f32) * 140.0 - (hill_offset % 140.0);
        ctx.line_to(hx, 350.0 + if i % 2 == 0 { -40.0 } else { -15.0 });
        ctx.line_to(hx + 70.0, 410.0);
    }
    ctx.line_to(SCREEN_W, SCREEN_H);
    ctx.close_path();
    ctx.fill();

    // ── 4. Platforms & Blocks (1.0x world speed) ─────────────────────
    for plat in &state.platforms {
        let draw_x = plat.x - cam;
        if draw_x + plat.w < 0.0 || draw_x > SCREEN_W {
            continue;
        }

        // Stone base
        ctx.set_fill_color(Color { r: 120, g: 53, b: 15, a: 255 });
        ctx.fill_rect(draw_x, plat.y, plat.w, plat.h);

        // Grass top trim
        ctx.set_fill_color(Color { r: 34, g: 197, b: 94, a: 255 });
        ctx.fill_rect(draw_x, plat.y, plat.w, 6.0);

        // Brick mortar highlights
        ctx.set_stroke_color(Color { r: 90, g: 40, b: 10, a: 255 });
        ctx.set_line_width(1.0);
        ctx.stroke_rect(draw_x, plat.y, plat.w, plat.h);
    }

    // Moving platforms
    for mp in &state.moving_platforms {
        let draw_x = mp.x - cam;
        if draw_x + mp.w >= 0.0 && draw_x <= SCREEN_W {
            ctx.set_fill_color(Color { r: 245, g: 158, b: 11, a: 255 }); // Amber moving platform
            ctx.fill_rect(draw_x, mp.y, mp.w, mp.h);

            ctx.set_fill_color(Color { r: 253, g: 230, b: 138, a: 255 });
            ctx.fill_rect(draw_x, mp.y, mp.w, 4.0);

            ctx.set_stroke_color(Color { r: 180, g: 83, b: 9, a: 255 });
            ctx.set_line_width(1.5);
            ctx.stroke_rect(draw_x, mp.y, mp.w, mp.h);
        }
    }

    // ── 5. Spinning Gold Coins ───────────────────────────────────────
    let spin = (state.tick as f32 * 0.12).cos();
    for coin in &state.coins {
        if !coin.collected {
            let cx = coin.x - cam;
            if cx >= -20.0 && cx <= SCREEN_W + 20.0 {
                let coin_w = (10.0 * spin).abs().max(2.0);
                // Outer gold
                ctx.set_fill_color(Color { r: 245, g: 158, b: 11, a: 255 });
                ctx.begin_path();
                ctx.arc(cx, coin.y, 10.0, 0.0, PI * 2.0, false);
                ctx.fill();

                // Inner bright gold shimmer
                ctx.set_fill_color(Color { r: 253, g: 224, b: 71, a: 255 });
                ctx.begin_path();
                ctx.arc(cx, coin.y, 7.0 * coin_w / 10.0, 0.0, PI * 2.0, false);
                ctx.fill();
            }
        }
    }

    // ── 6. Goal Flagpole & Castle at End ─────────────────────────────
    let flag_x = 3380.0 - cam;
    if flag_x >= -50.0 && flag_x <= SCREEN_W + 50.0 {
        // Pole
        ctx.set_fill_color(Color { r: 203, g: 213, b: 225, a: 255 });
        ctx.fill_rect(flag_x, 200.0, 6.0, 240.0);

        // Gold ball top
        ctx.set_fill_color(Color { r: 250, g: 204, b: 21, a: 255 });
        ctx.begin_path();
        ctx.arc(flag_x + 3.0, 195.0, 8.0, 0.0, PI * 2.0, false);
        ctx.fill();

        // Flag
        ctx.set_fill_color(Color { r: 239, g: 68, b: 68, a: 255 });
        ctx.begin_path();
        ctx.move_to(flag_x + 6.0, 205.0);
        ctx.line_to(flag_x + 45.0, 222.0);
        ctx.line_to(flag_x + 6.0, 240.0);
        ctx.close_path();
        ctx.fill();
    }

    // ── 7. Particles ────────────────────────────────────────────────
    for p in &state.particles {
        let px = p.x - cam;
        if px >= -10.0 && px <= SCREEN_W + 10.0 {
            ctx.set_fill_color(p.color);
            ctx.begin_path();
            ctx.arc(px, p.y, (p.life * 0.25).max(1.5), 0.0, PI * 2.0, false);
            ctx.fill();
        }
    }

    // ── 8. Player Character (Classic Retro Hero) ─────────────────────
    let px = state.player.x - cam;
    let py = state.player.y;
    let pw = state.player.w;
    let ph = state.player.h;

    // Body (Blue overalls)
    ctx.set_fill_color(Color { r: 37, g: 99, b: 235, a: 255 });
    ctx.fill_rect(px + 4.0, py + 14.0, pw - 8.0, ph - 20.0);

    // Shirt & Cap (Red)
    ctx.set_fill_color(Color { r: 220, g: 38, b: 38, a: 255 });
    ctx.fill_rect(px + 2.0, py + 10.0, pw - 4.0, 8.0); // Shirt
    ctx.fill_rect(px + 3.0, py, pw - 6.0, 7.0);       // Cap

    // Cap visor (facing direction)
    let visor_x = if state.player.facing_right { px + pw - 4.0 } else { px - 2.0 };
    ctx.fill_rect(visor_x, py + 3.0, 6.0, 4.0);

    // Head / Face
    ctx.set_fill_color(Color { r: 254, g: 205, b: 165, a: 255 });
    ctx.fill_rect(px + 5.0, py + 6.0, pw - 10.0, 7.0);

    // Eye
    ctx.set_fill_color(Color { r: 15, g: 23, b: 42, a: 255 });
    let eye_x = if state.player.facing_right { px + pw - 9.0 } else { px + 7.0 };
    ctx.fill_rect(eye_x, py + 7.0, 3.0, 3.0);

    // Running legs animation
    let leg_swing = (state.player.anim_timer.sin() * 5.0) as f32;
    ctx.set_fill_color(Color { r: 120, g: 53, b: 15, a: 255 }); // Brown shoes
    if state.player.on_ground {
        ctx.fill_rect(px + 4.0 + leg_swing, py + ph - 6.0, 7.0, 6.0);
        ctx.fill_rect(px + pw - 11.0 - leg_swing, py + ph - 6.0, 7.0, 6.0);
    } else {
        // Airborne jump pose
        ctx.fill_rect(px + 2.0, py + ph - 7.0, 7.0, 6.0);
        ctx.fill_rect(px + pw - 9.0, py + ph - 9.0, 7.0, 6.0);
    }

    // ── 9. Retro Arcade HUD ──────────────────────────────────────────
    // Semi-transparent top bar
    ctx.set_fill_color(Color { r: 15, g: 23, b: 42, a: 210 });
    ctx.fill_rect(0.0, 0.0, SCREEN_W, 42.0);

    ctx.set_font_css("bold 16px 'Courier New', monospace");
    ctx.set_fill_color(Color { r: 248, g: 250, b: 252, a: 255 });
    ctx.fill_text(&format!("SCORE: {:06}", state.score), 24.0, 26.0);

    ctx.set_fill_color(Color { r: 250, g: 204, b: 21, a: 255 });
    ctx.fill_text(&format!("COINS: {:02}/21", state.coins_collected), 220.0, 26.0);

    ctx.set_fill_color(Color { r: 52, g: 211, b: 153, a: 255 });
    ctx.fill_text("STAGE 1-1", 420.0, 26.0);

    ctx.set_font_css("12px system-ui, sans-serif");
    ctx.set_fill_color(Color { r: 148, g: 163, b: 184, a: 255 });
    ctx.fill_text("[A / D / Arrows] Move   [W / Up / Space] Jump   [R] Reset", 530.0, 26.0);

    // ── 10. Victory Banner ───────────────────────────────────────────
    if state.won {
        ctx.set_fill_color(Color { r: 15, g: 23, b: 42, a: 220 });
        ctx.fill_rect(180.0, 180.0, 440.0, 140.0);

        ctx.set_stroke_color(Color { r: 52, g: 211, b: 153, a: 255 });
        ctx.set_line_width(3.0);
        ctx.stroke_rect(180.0, 180.0, 440.0, 140.0);

        ctx.set_font_css("bold 28px system-ui, sans-serif");
        ctx.set_fill_color(Color { r: 52, g: 211, b: 153, a: 255 });
        ctx.fill_text("★ STAGE CLEAR! YOU WIN! ★", 215.0, 235.0);

        ctx.set_font_css("16px system-ui, sans-serif");
        ctx.set_fill_color(Color { r: 241, g: 245, b: 249, a: 255 });
        ctx.fill_text(&format!("Final Score: {}   •   Coins: {}", state.score, state.coins_collected), 270.0, 275.0);

        ctx.set_font_css("14px system-ui, sans-serif");
        ctx.set_fill_color(Color { r: 148, g: 163, b: 184, a: 255 });
        ctx.fill_text("Press [R] to Play Again!", 330.0, 305.0);
    }
}

fn main() {
    let document = BrowserDocument::new(osbrowser::load_html(
        "<!DOCTYPE html>\
        <html>\
        <head>\
            <title>osbrowser Retro Platformer</title>\
            <style>\
                * { box-sizing: border-box; }\
                body {\
                    margin: 0;\
                    padding: 0;\
                    background-color: #030712;\
                    display: flex;\
                    flex-direction: column;\
                    align-items: center;\
                    justify-content: center;\
                    min-height: 100vh;\
                    font-family: system-ui, -apple-system, sans-serif;\
                    color: #f8fafc;\
                    overflow: hidden;\
                }\
                h1 {\
                    margin: 0 0 10px 0;\
                    font-size: 22px;\
                    letter-spacing: 1px;\
                    color: #38bdf8;\
                    text-transform: uppercase;\
                }\
                canvas {\
                    box-shadow: 0 20px 50px rgba(0, 0, 0, 0.8), 0 0 0 2px #1e293b;\
                    border-radius: 8px;\
                    outline: none;\
                    background-color: #0f172a;\
                }\
            </style>\
        </head>\
        <body>\
            <h1>★ RetroRunner — osbrowser WHATWG Platformer ★</h1>\
            <canvas id=\"game_canvas\" width=\"800\" height=\"500\" tabindex=\"0\"></canvas>\
        </body>\
        </html>",
        800.0,
    ));

    let canvas_node = {
        let doc = document.write();
        doc.get_element_by_id("game_canvas")
            .expect("game_canvas element must exist")
    };

    let state = Arc::new(Mutex::new(GameState::new()));

    let mut browser = Browser::open(BrowserOptions {
        title: "RetroRunner - osbrowser Platformer".into(),
        ..Default::default()
    })
    .expect("failed to open browser");

    browser.attach_document(document.clone());
    browser.set_guest(PlatformerGame {
        state,
        canvas_node,
        document,
    });

    println!("===========================================================");
    println!("  RetroRunner Platformer is running in your browser tab!   ");
    println!("  Controls: [A/D or Arrows] Move  [W/Up/Space] Jump        ");
    println!("  Press [R] to Reset  •  Close browser tab to exit.        ");
    println!("===========================================================");

    browser.run().expect("browser loop ended");
}
