//! CSS-grid-like placement for AUIL `grid` nodes (P2).

use serde_json::Value;

#[derive(Clone, Debug)]
pub struct GridCell {
    pub child_index: usize,
    pub col: u32,
    pub row: u32,
    pub col_span: u32,
    pub row_span: u32,
}

#[derive(Clone, Debug)]
pub struct GridPlan {
    pub cols: u32,
    pub rows: u32,
    pub cells: Vec<GridCell>,
    pub cell_w: u32,
    pub row_heights: Vec<u32>,
    pub gap: u32,
    pub rtl: bool,
}

/// Build a row-major grid plan. `child_sizes` is `(width, height)` per child.
pub fn plan(
    cols: u32,
    gap: u32,
    avail_w: u32,
    child_sizes: &[(u32, u32)],
    child_props: &[Value],
    rtl: bool,
) -> GridPlan {
    let cols = cols.max(1);
    let cell_w = if cols == 0 {
        avail_w
    } else {
        avail_w
            .saturating_sub(gap * cols.saturating_sub(1))
            .saturating_div(cols)
            .max(1)
    };

    let mut occupied: Vec<bool> = Vec::new();
    let mut cells = Vec::new();
    let mut cursor = 0u32;

    for (i, ((_cw, _ch), props)) in child_sizes.iter().zip(child_props.iter()).enumerate() {
        let col_span = props
            .get("col_span")
            .or_else(|| props.get("cols"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .clamp(1, cols as u64) as u32;
        let row_span = props
            .get("row_span")
            .or_else(|| props.get("rows"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .max(1) as u32;

        // Find next free cell that fits col_span.
        loop {
            let row = cursor / cols;
            let col = cursor % cols;
            let need = ((row + row_span) * cols) as usize;
            if occupied.len() < need {
                occupied.resize(need, false);
            }
            let fits = col + col_span <= cols
                && (0..row_span).all(|dr| {
                    (0..col_span).all(|dc| {
                        let idx = ((row + dr) * cols + col + dc) as usize;
                        !occupied[idx]
                    })
                });
            if fits {
                for dr in 0..row_span {
                    for dc in 0..col_span {
                        let idx = ((row + dr) * cols + col + dc) as usize;
                        occupied[idx] = true;
                    }
                }
                cells.push(GridCell {
                    child_index: i,
                    col,
                    row,
                    col_span,
                    row_span,
                });
                break;
            }
            cursor += 1;
            if cursor > 10_000 {
                break;
            }
        }
        cursor += 1;
    }

    let rows = cells
        .iter()
        .map(|c| c.row + c.row_span)
        .max()
        .unwrap_or(0)
        .max(1);
    let mut row_heights = vec![0u32; rows as usize];
    for (cell, (_cw, ch)) in cells.iter().zip(child_sizes.iter()) {
        let per = (*ch).saturating_add(0);
        let span = cell.row_span.max(1);
        let share = per / span;
        for dr in 0..span {
            let r = (cell.row + dr) as usize;
            if r < row_heights.len() {
                row_heights[r] = row_heights[r].max(share.max(1));
            }
        }
        // Prefer child's full height in its first row when span==1.
        if cell.row_span == 1 {
            let r = cell.row as usize;
            row_heights[r] = row_heights[r].max(*ch);
        }
    }

    GridPlan {
        cols,
        rows,
        cells,
        cell_w,
        row_heights,
        gap,
        rtl,
    }
}

impl GridPlan {
    pub fn origin_of(&self, cell: &GridCell, origin_x: i32, origin_y: i32) -> (i32, i32) {
        let col = if self.rtl {
            self.cols
                .saturating_sub(cell.col + cell.col_span)
        } else {
            cell.col
        };
        let x = origin_x + (col * (self.cell_w + self.gap)) as i32;
        let mut y = origin_y;
        for r in 0..cell.row {
            y += (self.row_heights.get(r as usize).copied().unwrap_or(0) + self.gap) as i32;
        }
        (x, y)
    }

    pub fn size_of(&self, cell: &GridCell, child_w: u32, child_h: u32) -> (u32, u32) {
        let w = self.cell_w * cell.col_span
            + self.gap * cell.col_span.saturating_sub(1);
        let mut h = 0u32;
        for dr in 0..cell.row_span {
            h += self
                .row_heights
                .get((cell.row + dr) as usize)
                .copied()
                .unwrap_or(child_h);
            if dr + 1 < cell.row_span {
                h += self.gap;
            }
        }
        (w.max(child_w.min(w)), h.max(child_h.min(h.max(1))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn places_two_by_two() {
        let sizes = vec![(40, 20), (40, 20), (40, 20), (40, 20)];
        let props = vec![json!({}), json!({}), json!({}), json!({})];
        let p = plan(2, 8, 100, &sizes, &props, false);
        assert_eq!(p.cols, 2);
        assert_eq!(p.rows, 2);
        assert_eq!(p.cells.len(), 4);
        assert_eq!(p.cells[2].row, 1);
        assert_eq!(p.cells[2].col, 0);
    }

    #[test]
    fn rtl_mirrors_columns() {
        let sizes = vec![(40, 20), (40, 20)];
        let props = vec![json!({}), json!({})];
        let p = plan(2, 0, 80, &sizes, &props, true);
        let (x0, _) = p.origin_of(&p.cells[0], 0, 0);
        let (x1, _) = p.origin_of(&p.cells[1], 0, 0);
        assert!(x0 > x1, "rtl: first child should be on the right");
    }
}
