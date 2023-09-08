use super::TILES;
use super::{GridResult, Grid};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Neighbour {
    /// Or, 'No direction', checks the current cell during collision checks.
    Ident, 

    Up,
    UpLeft,
    UpRight,
    UpLeftSlip,
    UpRightSlip,
    Down,
    DownLeft,
    DownRight,
    DownLeftSlip,
    DownRightSlip,
    Left,
    Right,
}

use Neighbour::*;
pub const NOSLIP_NEIGHBOURS: &[Neighbour] = &[Ident, Up, UpLeft, UpRight, Down, DownLeft, DownRight, Left, Right];
pub const ALL_NEIGHBOURS: &[Neighbour] = &[Ident, Up, UpLeft, UpRight, UpRightSlip, UpLeftSlip, Down, DownLeft, DownLeftSlip, DownRight, DownRightSlip, Left, Right];

impl Neighbour {
    pub fn check_free(&self, grid: &Grid, x: usize, y: usize) -> Result<(usize, usize), GridResult> {
        let (w, h) = grid.get_wh();
        let (mx, my) = self.get_npos(x, y).ok_or(GridResult::OOB)?;
        if mx >= w { return Err(GridResult::OOB.into()) };
        if my >= h { return Err(GridResult::OOB.into()) };
        let tile = &TILES[grid[(mx, my)].index];
        //if grid[my][mx].index == grid[y][x].index { return Err(GridResult::Obstructed.into()); }
        use Neighbour::*;
        match self {
            Ident | Up | Down | Left | Right | UpLeftSlip | UpRightSlip | DownRightSlip | DownLeftSlip => {
                if !tile.solid { Ok((mx, my)) }
                else { Err(GridResult::Obstructed.into()) }
            }
            _ => {
                if tile.solid { Err(GridResult::Obstructed.into()) }
                else { 
                    let mut both_solid = true;
                    for c in self.components() {
                        let (mmx, mmy) = c.get_npos(x, y).ok_or(GridResult::OOB)?;

                        if mmx >= w { return Err(GridResult::OOB.into()) };
                        if mmy >= h { return Err(GridResult::OOB.into()) };
                        let tile = &TILES[grid[(mmx, mmy)].index];
                        both_solid &= tile.solid;
                    }
                    if both_solid { Err(GridResult::Obstructed.into()) }
                    else { Ok((mx, my)) }
                }
            }
        }
    }

    /// Get the 'new' position of the x and y coordinates given, or the position after the
    /// translation applied by this direction. 
    ///
    /// E.G: North => x, y + 1, etc...
    fn get_npos(&self, x: usize, y: usize) -> Option<(usize, usize)> {
        use Neighbour::*;
        Some(match self {
            Ident       => (x, y),

            Down        => (x, y + 1),
            Right       => (x + 1, y),
            Up          => (x, y.checked_sub(1)?),
            Left        => (x.checked_sub(1)?, y),
            UpLeft | UpLeftSlip         => (x.checked_sub(1)?, y.checked_sub(1)?),
            DownLeft | DownLeftSlip     => (x.checked_sub(1)?, y + 1),
            UpRight | UpRightSlip       => (x + 1, y.checked_sub(1)?),
            DownRight | DownRightSlip   => (x + 1, y + 1),
        })
    }

    pub fn components(&self) -> &[Neighbour] {
        use Neighbour::*;
        match self {
            // FIXME: is 'Ident' made up of every direction, or no direction?
            DownLeft | DownLeftSlip => &[Down, Left],
            DownRight | DownRightSlip => &[Down, Right],
            UpRight | UpRightSlip => &[Up, Right],
            UpLeft | UpLeftSlip => &[Up, Left],
            _ => &[]
        }
    }
}
