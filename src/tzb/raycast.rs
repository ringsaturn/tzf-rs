//! Segment raycast for the in-place reader.
//!
//! Direct port of the ray-casting function that `geometry-rs` (and the Go
//! `internal/geom` package) inherited from `github.com/tidwall/geojson`
//! (MIT licence), so every lookup mechanism in this crate shares one boundary
//! behavior. `geometry-rs` 0.5 exports `raycast` but keeps the result fields
//! private, hence this local copy; parity between the in-place reader and the
//! geometry-rs-backed expanded finder is asserted by the mechanism-parity
//! tests.
//!
//! Edge cases handled:
//! - Horizontal and vertical segments.
//! - Points that land exactly on a vertex: `py` is nudged via `next_up` so
//!   the vertex is counted at most once, matching the winding convention of
//!   the original geojson library (`math.Nextafter(py, +Inf)` in Go).
//! - Collinear points are reported as "on" rather than "inside".

use geometry_rs::Point;

/// Tests whether a leftward horizontal ray from `p` crosses segment (a, b).
/// Returns `(inside, on)`: `inside` reports a crossing, `on` reports that `p`
/// lies on the segment.
#[allow(clippy::similar_names)]
pub(crate) fn raycast_seg(a: Point, b: Point, p: Point) -> (bool, bool) {
    let mut py = p.y;

    // Quick Y-range rejection.
    if a.y < b.y {
        if py < a.y || py > b.y {
            return (false, false);
        }
    } else if a.y > b.y && (py < b.y || py > a.y) {
        return (false, false);
    }

    // Detect if p lies on the segment before the raycast nudge.
    if a.y == b.y {
        // horizontal segment
        if a.x == b.x {
            // degenerate (single point)
            if p.x == a.x && py == a.y {
                return (false, true);
            }
            return (false, false);
        }
        if py == b.y {
            if a.x < b.x {
                if p.x >= a.x && p.x <= b.x {
                    return (false, true);
                }
            } else if p.x >= b.x && p.x <= a.x {
                return (false, true);
            }
        }
    }
    if a.x == b.x && p.x == b.x {
        // vertical segment
        if a.y < b.y {
            if py >= a.y && py <= b.y {
                return (false, true);
            }
        } else if py >= b.y && py <= a.y {
            return (false, true);
        }
    }
    // General collinearity check. Division by zero yields Inf/NaN; NaN != NaN
    // and Inf != finite, so the comparison safely returns false in those cases.
    if (p.x - a.x) / (b.x - a.x) == (py - a.y) / (b.y - a.y) {
        return (false, true);
    }

    // Nudge py off any vertex to avoid double-counting shared polygon vertices.
    while py == a.y || py == b.y {
        py = py.next_up();
    }

    // Re-check Y bounds after nudge.
    if a.y < b.y {
        if py < a.y || py > b.y {
            return (false, false);
        }
    } else if py < b.y || py > a.y {
        return (false, false);
    }

    // X-axis shortcuts: if p.x is clearly to the right or left of both
    // endpoints, the crossing result is trivial.
    if a.x > b.x {
        if p.x >= a.x {
            return (false, false);
        }
        if p.x <= b.x {
            return (true, false);
        }
    } else {
        if p.x >= b.x {
            return (false, false);
        }
        if p.x <= a.x {
            return (true, false);
        }
    }

    // Slope comparison to determine which side of the segment p lies on.
    if a.y < b.y {
        if (py - a.y) / (p.x - a.x) >= (b.y - a.y) / (b.x - a.x) {
            return (true, false);
        }
    } else if (py - b.y) / (p.x - b.x) >= (a.y - b.y) / (a.x - b.x) {
        return (true, false);
    }
    (false, false)
}
