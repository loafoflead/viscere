use super::{Canvas2, TILES};
use sdl2::rect::Rect;
use std::fmt;

mod neighbour;
pub use neighbour::*;

const G: f32 = 2.0;
const DECELERATION_Y: f32 = 1.0;
const DECELERATION_X: f32 = 0.7;
const SLOWEST_X_SPEED: f32 = 0.2;

pub const CURS_SMALLEST : usize = 1;

pub const TILE_WIDTH    : usize = 10;
pub const TILE_HEIGHT   : usize = 10;

use std::ops::*;

#[derive(Debug, thiserror::Error)]
pub enum GridCheck {
    /// Out of bounds
    OOB,
    Obstructed,
}

impl fmt::Display for GridCheck {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "go and fuck yourself")
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Vec2(pub f32, pub f32);

impl Vec2 {
    pub const ZERO: Vec2 = Vec2(0.0, 0.0);
    const MAX_LINE_MIDPOINTS: usize = 5000;

    fn lerp(v1: &Self, v2: &Self, t: f32) -> Self {
        Vec2(lerp(v1.0, v2.0, t), lerp(v1.1, v2.1, t))
    }

    fn dist(&self, r: &Self) -> f32 {
        ((r.0 - self.0).powf(2.0) + (r.1 - self.1).powf(2.0)).sqrt() 
    }

    fn round(&self) -> (isize, isize) {
        (self.0.round() as isize, self.1.round() as isize)
    }

    fn line(p1: Vec2, p2: Vec2) -> Vec<(isize, isize)> {
        let mut pts = vec![];

        let n = ((p1.dist(&p2) as usize).clamp(1, Self::MAX_LINE_MIDPOINTS) as f32 * 1.0) as usize;
        for s in 0..n {
            let t = if n == 0 { 0.0 } else { s as f32 / n as f32 };
            pts.push(Vec2::lerp(&p1, &p2, t).round());
        }
        pts
    }
}

impl std::ops::Add<Vec2> for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Self::Output {
        Self (
            self.0 + rhs.0,
            self.1 + rhs.1
        )
    }
}

impl std::ops::Sub<Vec2> for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Self::Output {
        Self (
            self.0 - rhs.0,
            self.1 - rhs.1
        )
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Self::Output {
        Self (
            self.0 * rhs,
            self.1 * rhs
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TileIdType {
    Static,
    Dynamic,
}

#[derive(Clone, Debug)]
pub struct Tile {
    index: TileIndex,
    sort: TileIdType,
    vel: Vec2, 
    acc: Vec2,
    updated: bool,
}

impl Tile {
    fn new(index: TileIndex, sort: TileIdType) -> Self {
        Self {
            index, sort, vel: Vec2(0.0, 0.0), acc: Vec2(0.0, 0.0), updated: false
        }
    }
}

// FIXME: stack overflow when too many elems, move grid to the heap
pub struct Grid<const W: usize, const H: usize> {
    grid: TileGrid<W, H>,
}

type TileIndex = usize;
pub type TileGrid<const W: usize, const H: usize> = [[Tile; W]; H];

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start * (1.0 - t) + end * t
}

impl<const W: usize, const H: usize> Grid<W, H> {

    pub fn new() -> Self {
        Grid {
            grid: std::array::from_fn(|_| std::array::from_fn(|_| Tile::new(0, TILES[0].sort)))
        }
    }

    

    pub fn draw(&mut self, canvas: &mut Canvas2) {
        for y in 0..H {
            for x in 0..W {
                let rect = Rect::new(x as i32 * TILE_WIDTH as i32, y as i32 * TILE_HEIGHT as i32, TILE_WIDTH as u32, TILE_HEIGHT as u32);
                canvas.set_draw_color(if TILES.len()-1 >= self.grid[y][x].index { TILES[self.grid[y][x].index].colour } else { (255, 0, 0) });
                let _ = canvas.fill_rect(rect);
            }
        }
    }

    pub fn get_cols_in_rect(&self, rect: Rect) -> Option<Vec<Rect>> {
        if rect.x < 0 || rect.y < 0 || rect.x as usize / TILE_WIDTH >= W || rect.y as usize / TILE_HEIGHT >= H { return None; }

        let x_range = rect.x as usize / TILE_WIDTH..(rect.x as usize + rect.w as usize + TILE_WIDTH - 1) / TILE_WIDTH;
        let y_range = rect.y as usize / TILE_HEIGHT..(rect.y as usize + rect.h as usize + TILE_HEIGHT - 1) / TILE_HEIGHT;

        let mut res = vec!();

        for y in y_range {
            for x in x_range.clone() {
                if TILES[self.grid[y][x].index].solid {
                    let rect = Rect::new(
                        x as i32 * TILE_WIDTH as i32, 
                        y as i32 * TILE_HEIGHT as i32, 
                        TILE_WIDTH as u32,
                        TILE_HEIGHT as u32,
                    );
                    res.push(rect);
                }
            }
        }

        Some(res)
    }

    fn find_free(&self, x: usize, y: usize, neighbours: &[Neighbour]) -> Result<(usize, usize), GridCheck> {
        if x >= W || y >= H { return Err(GridCheck::OOB.into()); }

        if neighbours == &[Neighbour::Ident] {
            return neighbours[0].check_free(&self.grid, x, y);
        }
        
        let mut oob = true;
        for n in neighbours {
            match n.check_free(&self.grid, x, y) {
                Ok(r) => {
                    if n.components().contains(&Neighbour::Left) || n.components().contains(&Neighbour::Right) {
                        return Ok(r);
                    }
                    else { return Ok(r); }
                }
                Err(GridCheck::OOB) => {
                    oob &= true;
                }
                _ => {
                    oob &= false;
                }
            }
            //if let Some(r) = n.check_free(&self.grid, x, y) { return Ok(r) };
        }

        if oob == true {
            Err(GridCheck::OOB)
        }
        else {
            Err(GridCheck::Obstructed.into())
        }
    }

    fn swap(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let prev = self.grid[y2][x2].clone();
        self.grid[y2][x2] = self.grid[y1][x1].clone();
        self.grid[y1][x1] = prev;
    }

    pub fn count_dynamic_updates(&self) -> (usize, Vec<(usize, usize)>) {
        let mut count = 0usize;
        let mut l = vec!();
        for x in 0..W {
            for y in (0..H).rev() {
                let tile_id = &TILES[self.grid[y][x].index];
                if let TileIdType::Dynamic = tile_id.sort {
                    if let Ok((nx, _ny)) = self.find_free(x, y, tile_id.neighbours) {
                                // dont update if the new pos has a diff x but no x vel or yvel
                        if !(nx != x && (self.grid[y][x].vel.0 == 0.0 && self.grid[y][x].vel.1 == 0.0)) {
                            count += 1;
                            l.push((x, y));
                        }
                    }
                }
            }
        }
        (count, l)
    }

    pub fn update(&mut self) {
        for x in 0..W {
            for y in (0..H).rev() {
                if self.grid[y][x].updated == true { 
                    self.grid[y][x].updated = false;
                    continue; 
                }
                let tile_id = &TILES[self.grid[y][x].index];
                if tile_id.gravity {
                    self.update_static_tile(x, y, tile_id);
                    /*match tile_id.sort {
                        TileIdType::Static => {
                            if let Ok((nx, ny)) = self.find_free(x, y, tile_id.neighbours) {
                                self.swap(x, y, nx, ny);
                                self.grid[ny][nx].updated = true;
                            }
                            else if let Err(GridCheck::OOB) = self.find_free(x, y, tile_id.neighbours) {
                                self.set(x, y, 0, CURS_SMALLEST);
                            }
                            else {} // do nothing, the block won't move by default
                        }
                        TileIdType::Dynamic => {
                            if let Ok((_nx, _ny)) = self.find_free(x, y, tile_id.neighbours) {
                                // dont update if the new pos has a diff x but no x vel or yvel
                                //if !(nx != x && (self.grid[y][x].vel.0.abs() <= 1.0 && self.grid[y][x].vel.1 == 0.0)) {
                                    self.update_dynamic_tile_grid(x, y, tile_id);
                                //}
                            }
                            else if let Err(GridCheck::OOB) = self.find_free(x, y, tile_id.neighbours) {
                                self.set(x, y, 0, CURS_SMALLEST);
                            }
                        }
                    }*/
                }
            }
        }
    }

    fn update_static_tile(&mut self, x: usize, y: usize, tile_id: &crate::TileId) {
        if let Ok((nx, ny)) = self.find_free(x, y, tile_id.neighbours) {
            self.swap(x, y, nx, ny);
            self.grid[ny][nx].updated = true;
        }
        else if let Err(GridCheck::OOB) = self.find_free(x, y, tile_id.neighbours) {
            self.set(x, y, 0, CURS_SMALLEST);
        }
        else {} // do nothing, the block won't move by default
    }

    fn update_dynamic_tile_grid(&mut self, x: usize, y: usize, tile_id: &crate::TileId) {
        let tile = &mut self.grid[y][x];

        // Apply acceleration to velocity
        tile.acc = Vec2(0.0, G * tile_id.weight);
        tile.vel = tile.vel + tile.acc;

        // Apply (air resistance?) deceleration
        tile.vel.1 *= DECELERATION_Y;
        tile.vel.0 *= DECELERATION_X;
        if tile.vel.0.abs() < SLOWEST_X_SPEED { tile.vel.0 = 0.0; }

        let origin = Vec2(x as f32, y as f32);
        let target = origin + tile.vel;

        if target == origin { return; }
    
        let mut nx = x; let mut ny = y;

        for (i, (ptx, pty)) in Vec2::line(origin, target).into_iter().enumerate() {
            if i == 0 { continue; }
            if ptx < 0 || pty < 0 {
                self.set(x, y, 0, CURS_SMALLEST);
                return;
            }
            match self.find_free(ptx as usize, pty as usize, [&[Neighbour::Ident], tile_id.neighbours].concat().as_slice()) {
                Ok((nnx, nny)) => {
                    // We want to check for every point across this line that the free spot is the
                    // point we're currently on, which it should be unless we hit a floor/ceiling
                    // with a free spot adjacent depending on the tile's neighbour rules 
                    if (nnx, nny) != (ptx as usize, pty as usize) {
                        // eprintln!("Found alternative tile placement: {:?}, as opposed to {:?}", (nnx, nny), (ptx, pty));
                        // If they don't match, then the free is adjacent somehow to the point
                        // along the line. In this case, we will want to place our tile there and
                        // quit the loop early, to resolve this in a later update stage by keeping
                        // momentum
                        // FIXME: do this using recursion instead
                        self.swap(x, y, nnx, nny);
                        self.grid[ny][nx].updated = true;
                        return;
                    }
                    nx = nnx; ny = nny;
                }
                Err(GridCheck::OOB) => {
                    self.set(x, y, 0, CURS_SMALLEST);
                    return;
                }
                Err(GridCheck::Obstructed) => {
                    // Here we can assume that no matter the velocity of the tile, it has no free
                    // squares to move to, so we will reset it's velocity and set it to the most
                    // recent free spot. This should only really happen when a single tile falls
                    // directly into a spot it can't move from, like a grain of sand falling in the
                    // middle of a three tile solid platform
                    let tile = &mut self.grid[ny][nx];
                    tile.vel = Vec2::ZERO;
                    break;
                }
            }
        }
        self.swap(x, y, nx, ny);
        self.grid[ny][nx].updated = true;
    }

    fn update_dynamic_tile_phys(&mut self, x: usize, y: usize, tile_id: &crate::TileId) {
        let tile = &mut self.grid[y][x];

        // Apply acceleration to velocity
        tile.acc = Vec2(0.0, G * tile_id.weight);
        tile.vel = tile.vel + tile.acc;

        // Apply (air resistance?) deceleration
        tile.vel.1 *= DECELERATION_Y;
        tile.vel.0 *= DECELERATION_X;
        if tile.vel.0.abs() < SLOWEST_X_SPEED { tile.vel.0 = 0.0; }

        // eprintln!("INFO: Dynamic tile update: Tile: {:?}", tile);
        
        let origin = Vec2(x as f32, y as f32);
        let target = origin + tile.vel;

        if target == origin { return; }

        let mut nx = x; let mut ny = y;
        //eprintln!("INFO: target: {:?}, origin: {:?}, tot: {tot_steps}", target, origin);
        for (i, (ptx, pty)) in Vec2::line(origin, target).into_iter().enumerate() {
            if i == 0 { continue; }
            if ptx < 0 || pty < 0 {
                self.set(x, y, 0, CURS_SMALLEST);
                return;
            }
            match self.find_free(ptx as usize, pty as usize, &[Neighbour::Ident]) {
                Ok((f,g)) => { nx = f; ny = g; },

                Err(GridCheck::OOB) => {
                    self.set(x, y, 0, CURS_SMALLEST);
                    return;
                }

                Err(GridCheck::Obstructed) => {
                    /*if i != 0 {
                        let t = (i-1) as f32 / tot_steps as f32;
                        let collision_pt = Vec2::lerp(&origin, &target, t);

                        let tile = &mut self.grid[y][x];
                        tile.vel = Vec2::ZERO;
                        self.swap(x, y, collision_pt.0 as usize, collision_pt.1 as usize);
                        break;
                    }*/


                    // Assume a 45° angle for all collisions

                    if self.grid[y][x].vel.1 != 0.0 {
                        if nx != 0 && !self.get_solidity(nx - 1, ny+1) {
                            let tile = &mut self.grid[y][x];
                            tile.vel.0 = -tile.vel.1/2.0;
                        }
                        else if !self.get_solidity(nx + 1, ny+1) {
                            let tile = &mut self.grid[y][x];
                            tile.vel.0 = tile.vel.1/2.0;
                        }
                        let tile = &mut self.grid[y][x];
                        tile.vel.1 = 0.0;
                    }

                    break;
                }
            }
        }
        self.swap(x, y, nx, ny);
        self.grid[ny][nx].updated = true;
    }

    pub fn get_solidity(&self, x: usize, y: usize) -> bool {
        if x >= W || y >= H { return false; }
        TILES[self.grid[y][x].index].solid  
    }

    pub fn set(&mut self, mut x: usize, mut y: usize, tile: TileIndex, size: usize) {
        if size == 1 {
            self.grid[y.clamp(0, H)][x.clamp(0, W)] = Tile::new( tile, TILES[tile].sort);
            return;
        }
        if x.checked_sub(size/2).is_none() { x = size/2; }
        if y.checked_sub(size/2).is_none() { y = size/2; }
        for y in y-size/2..y+size/2 {
            for x in x-size/2..x+size/2 {
                self.grid[y.clamp(0, H-1)][x.clamp(0, W-1)] = Tile::new( tile, TILES[tile].sort);
            }
        }
    }

    pub fn clear(&mut self) {
        for y in 0..H {
            for x in 0..W {
                self.grid[y][x] = Tile::new(0, TILES[0].sort);
            }
        }
    }
}
