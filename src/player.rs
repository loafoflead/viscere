use super::*;

const PLAYER_WIDTH      : u32 = TILE_WIDTH as u32;
const PLAYER_HEIGHT     : u32 = TILE_WIDTH as u32 * 2u32;

const GROUNDED_COLLIDER_WIDTH: u32 = PLAYER_WIDTH;
const GROUNDED_COLLIDER_HEIGHT: u32 = PLAYER_HEIGHT / 3;

const PLAYER_DECELERATION: f32 = 0.85;

const GROUND_FRICTION: f32 = 0.65;
const HORIZ_AIR_DECELERATION: f32 = 0.4;
const VERT_AIR_DECELERATION: f32 = 0.85;

const GRAVITY: f32 = 3.0;
pub const PLAYER_HORIZONTAL_MOVEMENT_SPEED: f32 = 5.0;

pub const MAXJUMP: f32 = 6.0;

const PLAYER_COLOUR     : Color = Color::RGB(10, 50, 200);


#[derive(Default, Debug)]
pub struct Player {
    pub pos: Vec2,
    vel: Vec2,
    acc: Vec2
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self { pos: Vec2(x, y), ..Default::default() }
    }

    pub fn draw(&self, canvas: &mut Canvas2) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let rect = Rect::new(self.pos.0 as i32, self.pos.1 as i32, PLAYER_WIDTH, PLAYER_HEIGHT);
        canvas.set_draw_color(PLAYER_COLOUR);
        canvas.fill_rect(rect)?;    
        Ok(())
    }

    fn _debug_intersections(&self, canvas: &mut Canvas2, grid: &Grid) {
        if let Some(rs) = grid.get_cols_in_rect(self.rect()) {
            for r in rs {
                canvas.set_draw_color(DEBUG_DRAW_COLOUR);
                canvas.fill_rect(r).unwrap();
            }
        }
        if let Some(cols) = grid.get_cols_in_rect(self.rect()) {
            for col in cols {
                let player_rect = self.rect();
                let col_obj_rect = col;

                let intersection = player_rect.intersection(col_obj_rect).unwrap();

                canvas.set_draw_color(DEBUG_DRAW_COLOUR);
                canvas.fill_rect(intersection).unwrap();
            }
        }
    }

    fn rect(&self) -> Rect {
        Rect::new(self.pos.0.ceil() as i32, self.pos.1.ceil() as i32, PLAYER_WIDTH, PLAYER_HEIGHT)
    }

    pub fn is_grounded(&self, grid: &Grid) -> bool {
        let col_rect = Rect::new(self.pos.0 as i32 + PLAYER_WIDTH as i32/8, self.pos.1 as i32 + PLAYER_HEIGHT as i32, GROUNDED_COLLIDER_WIDTH - PLAYER_WIDTH/8, GROUNDED_COLLIDER_HEIGHT);
        if let Some(cols) = grid.get_cols_in_rect(col_rect) {
            if cols.len() > 0 {
                true
            }
            else { false }
        }
        else {
            false
        }
    }

    pub fn update(&mut self, grid: &Grid) {
        let prev = self.pos;
        self.acc.1 = GRAVITY; 
        self.vel = self.vel + self.acc;
        self.vel.1 = self.vel.1 * VERT_AIR_DECELERATION;
        if self.is_grounded(grid) {
            self.vel.0 *= GROUND_FRICTION;
        }
        else {
            self.vel.0 *= HORIZ_AIR_DECELERATION;
        }
        self.acc.0 *= PLAYER_DECELERATION;

        self.pos = self.pos + self.vel;
        'substep: for (i, pt) in Vec2::linef32(prev, self.pos).into_iter().enumerate() {
            self.pos = pt;
            if let Some(cols) = grid.get_cols_in_rect(self.rect()) {
                let len = cols.len();
                for col in cols {
                    let player_rect = self.rect();
                    let col_obj_rect = col;

                    let Some(intersection) = player_rect.intersection(col_obj_rect) else { continue; };

                    let pcentre: Vec2 = player_rect.center().into();
                    let ccentre = col_obj_rect.center().into();

                    let p_to_obj = pcentre - ccentre;
                    // eprintln!("{:?}", (ccentre - pcentre));
                    
                    let (x_part, y_part) = if intersection.w == intersection.h {
                        let p_to_obj = prev - self.pos;

                        (p_to_obj.0.signum() * intersection.w as f32, p_to_obj.1.signum() * intersection.h as f32)
                    } else {
                        let x_part = if intersection.w < intersection.h { p_to_obj.0.signum() * intersection.w as f32 } else { 0.0 };
                        let y_part = if intersection.h < intersection.w { p_to_obj.1.signum() * intersection.h as f32 } else { 0.0 };
                        (x_part, y_part)
                    };

                    self.pos = self.pos + Vec2(x_part, y_part);

                    if intersection.w < intersection.h { self.vel.0 = 0.0; }
                    if intersection.h < intersection.w { self.vel.1 = 0.0; }
                }
                if len > 0 && i > 1 {
                    break 'substep;
                }
            }
        }
    }

    pub fn move_x(&mut self, acc: f32) {
        self.vel.0 += acc;
    }

    pub fn move_y(&mut self, acc: f32) {
        self.vel.1 += acc;
    }
}
