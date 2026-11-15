//! Lane layout for the structured History graph: one row per commit, with
//! the lines to its parents and from its children drawn on that row.
//!
//! Commits arrive newest first with children before parents (`git log
//! --topo-order`). Each lane waits for one commit; a commit takes the lane
//! that waits for it, hands it to its first parent, and opens or joins a
//! lane for each other parent. The layout keeps its lanes between calls, so
//! later pages continue the same graph.

use crate::commit_log::Commit;

/// One screen cell of the graph. Lanes sit in even columns; odd columns
/// carry horizontal lines between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    Space,
    /// The commit.
    Node,
    /// A lane passing through.
    Vertical,
    Horizontal,
    /// A horizontal line crossing a lane.
    Cross,
    /// A line from the left turning down to a new lane: `╮`.
    DownFromLeft,
    /// A line from the right turning down to a new lane: `╭`.
    DownFromRight,
    /// A lane from above ending in a line to the left: `╯`.
    UpToLeft,
    /// A lane from above ending in a line to the right: `╰`.
    UpToRight,
    /// A line from the left joining a lane that continues: `┤`.
    JoinFromLeft,
    /// A line from the right joining a lane that continues: `├`.
    JoinFromRight,
}

impl Glyph {
    pub fn unicode(self) -> char {
        match self {
            Glyph::Space => ' ',
            Glyph::Node => '●',
            Glyph::Vertical => '│',
            Glyph::Horizontal => '─',
            Glyph::Cross => '┼',
            Glyph::DownFromLeft => '╮',
            Glyph::DownFromRight => '╭',
            Glyph::UpToLeft => '╯',
            Glyph::UpToRight => '╰',
            Glyph::JoinFromLeft => '┤',
            Glyph::JoinFromRight => '├',
        }
    }

    pub fn ascii(self) -> char {
        match self {
            Glyph::Space => ' ',
            Glyph::Node => '*',
            Glyph::Vertical | Glyph::JoinFromLeft | Glyph::JoinFromRight => '|',
            Glyph::Horizontal => '-',
            Glyph::Cross => '+',
            Glyph::DownFromLeft | Glyph::DownFromRight => '.',
            Glyph::UpToLeft | Glyph::UpToRight => '\'',
        }
    }
}

/// A cell and the colour of the lane it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub glyph: Glyph,
    /// An index into the lane palette; stable for a lane's whole life.
    pub color: usize,
}

/// The graph cells for one commit's row, two columns per lane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphRow {
    pub cells: Vec<Cell>,
    /// The lane holding the commit.
    pub lane: usize,
}

#[derive(Clone, Debug)]
struct Lane {
    /// The commit this lane waits for.
    waiting_for: String,
    color: usize,
}

/// Assigns lanes to commits in order, keeping them across pages.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    lanes: Vec<Option<Lane>>,
    next_color: usize,
}

/// How another lane connects to the commit on its row.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Link {
    /// A child's lane ends here.
    Ends,
    /// A new lane starts here for a parent.
    Starts,
    /// The line joins a lane that continues to a shared parent.
    Joins,
}

impl Layout {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lanes still waiting for a commit. Zero after a whole history.
    pub fn open_lanes(&self) -> usize {
        self.lanes.iter().flatten().count()
    }

    /// Lay out the next commit.
    pub fn push(&mut self, commit: &Commit) -> GraphRow {
        let waiting = |lane: &Option<Lane>| {
            lane.as_ref()
                .is_some_and(|lane| lane.waiting_for == commit.hash)
        };
        let before: Vec<bool> = self.lanes.iter().map(Option::is_some).collect();
        // The leftmost lane waiting for this commit holds it; a new branch
        // tip takes the first free lane.
        let lane = match self.lanes.iter().position(waiting) {
            Some(lane) => lane,
            None => {
                let color = self.new_color();
                self.place(
                    Lane {
                        waiting_for: commit.hash.clone(),
                        color,
                    },
                    &[],
                )
            }
        };
        let color = self.lanes[lane].as_ref().map_or(0, |lane| lane.color);
        let mut links: Vec<(usize, Link, usize)> = Vec::new();
        // Other children's lanes end here.
        for index in 0..self.lanes.len() {
            if index != lane && waiting(&self.lanes[index]) {
                let ended = self.lanes[index].take().map_or(0, |lane| lane.color);
                links.push((index, Link::Ends, ended));
            }
        }
        // The first parent continues this lane.
        self.lanes[lane] = commit.parents.first().map(|parent| Lane {
            waiting_for: parent.clone(),
            color,
        });
        for parent in commit.parents.iter().skip(1) {
            let existing = self.lanes.iter().enumerate().position(|(index, other)| {
                index != lane
                    && other
                        .as_ref()
                        .is_some_and(|other| &other.waiting_for == parent)
            });
            match existing {
                Some(index) => {
                    let joined = self.lanes[index].as_ref().map_or(0, |lane| lane.color);
                    links.push((index, Link::Joins, joined));
                }
                None => {
                    let new_color = self.new_color();
                    // Lanes freed on this row stay free until the next one,
                    // so no cell both ends and starts a lane.
                    let freed: Vec<usize> = links.iter().map(|&(index, _, _)| index).collect();
                    let index = self.place(
                        Lane {
                            waiting_for: parent.clone(),
                            color: new_color,
                        },
                        &freed,
                    );
                    links.push((index, Link::Starts, new_color));
                }
            }
        }
        let row = self.draw(lane, color, &before, &links);
        while self.lanes.last().is_some_and(Option::is_none) {
            self.lanes.pop();
        }
        row
    }

    fn new_color(&mut self) -> usize {
        let color = self.next_color;
        self.next_color += 1;
        color
    }

    /// Put `lane` in the first free slot not in `avoid`, or a new one at
    /// the right.
    fn place(&mut self, lane: Lane, avoid: &[usize]) -> usize {
        let free = self
            .lanes
            .iter()
            .enumerate()
            .position(|(index, slot)| slot.is_none() && !avoid.contains(&index));
        match free {
            Some(index) => {
                self.lanes[index] = Some(lane);
                index
            }
            None => {
                self.lanes.push(Some(lane));
                self.lanes.len() - 1
            }
        }
    }

    /// The row's cells. `before` marks lanes occupied above the row.
    fn draw(
        &self,
        lane: usize,
        color: usize,
        before: &[bool],
        links: &[(usize, Link, usize)],
    ) -> GraphRow {
        let width = links
            .iter()
            .map(|&(index, _, _)| index)
            .chain([lane])
            .chain((0..self.lanes.len()).filter(|&i| self.lanes[i].is_some()))
            .max()
            .unwrap_or(0)
            + 1;
        let blank = Cell {
            glyph: Glyph::Space,
            color: 0,
        };
        let mut cells = vec![blank; width * 2 - 1];
        // Lanes passing through: occupied above and below, not linked.
        for index in 0..width {
            let linked = links.iter().any(|&(other, _, _)| other == index);
            let above = before.get(index).copied().unwrap_or(false);
            let below = self.lanes.get(index).is_some_and(Option::is_some);
            if index != lane && !linked && above && below {
                let color = self.lanes[index].as_ref().map_or(0, |lane| lane.color);
                cells[index * 2] = Cell {
                    glyph: Glyph::Vertical,
                    color,
                };
            }
        }
        for &(index, link, link_color) in links {
            let right = index > lane;
            let glyph = match (link, right) {
                (Link::Ends, true) => Glyph::UpToLeft,
                (Link::Ends, false) => Glyph::UpToRight,
                (Link::Starts, true) => Glyph::DownFromLeft,
                (Link::Starts, false) => Glyph::DownFromRight,
                (Link::Joins, true) => Glyph::JoinFromLeft,
                (Link::Joins, false) => Glyph::JoinFromRight,
            };
            cells[index * 2] = Cell {
                glyph,
                color: link_color,
            };
            let (from, to) = if right {
                (lane * 2 + 1, index * 2)
            } else {
                (index * 2 + 1, lane * 2)
            };
            for cell in &mut cells[from..to] {
                cell.glyph = match cell.glyph {
                    Glyph::Vertical | Glyph::Cross => Glyph::Cross,
                    Glyph::Space | Glyph::Horizontal => Glyph::Horizontal,
                    // Another link's corner: keep it.
                    other => other,
                };
                if cell.glyph == Glyph::Horizontal {
                    cell.color = link_color;
                }
            }
        }
        cells[lane * 2] = Cell {
            glyph: Glyph::Node,
            color,
        };
        GraphRow { cells, lane }
    }
}

impl GraphRow {
    /// The row as text, for tests and the plain-text views.
    pub fn text(&self, ascii: bool) -> String {
        self.cells
            .iter()
            .map(|cell| {
                if ascii {
                    cell.glyph.ascii()
                } else {
                    cell.glyph.unicode()
                }
            })
            .collect::<String>()
            .trim_end()
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(hash: &str, parents: &[&str]) -> Commit {
        Commit {
            hash: hash.to_owned(),
            parents: parents.iter().map(|p| (*p).to_owned()).collect(),
            refs: Vec::new(),
            author: String::new(),
            time: 0,
            subject: hash.to_owned(),
        }
    }

    /// The graph for `commits`, one line per commit.
    fn graph(commits: &[Commit]) -> Vec<String> {
        let mut layout = Layout::new();
        commits
            .iter()
            .map(|commit| layout.push(commit).text(false))
            .collect()
    }

    #[test]
    fn a_linear_history_is_one_lane() {
        let commits = [commit("c", &["b"]), commit("b", &["a"]), commit("a", &[])];
        assert_eq!(graph(&commits), ["●", "●", "●"]);
    }

    #[test]
    fn a_merge_opens_a_lane_that_ends_at_the_fork_point() {
        // m merges f into the line b; both come from a.
        let commits = [
            commit("m", &["b", "f"]),
            commit("f", &["a"]),
            commit("b", &["a"]),
            commit("a", &[]),
        ];
        assert_eq!(graph(&commits), ["●─╮", "│ ●", "● │", "●─╯"]);
    }

    #[test]
    fn an_octopus_merge_opens_a_lane_per_parent() {
        let commits = [
            commit("m", &["a", "x", "y"]),
            commit("y", &["a"]),
            commit("x", &["a"]),
            commit("a", &[]),
        ];
        assert_eq!(graph(&commits), ["●─╮─╮", "│ │ ●", "│ ● │", "●─╯─╯"]);
    }

    #[test]
    fn a_long_lived_branch_keeps_its_lane_and_colour() {
        // main: m3 m2 m1 base; dev: d2 d1 base, interleaved by time.
        let commits = [
            commit("d2", &["d1"]),
            commit("m3", &["m2"]),
            commit("d1", &["base"]),
            commit("m2", &["m1"]),
            commit("m1", &["base"]),
            commit("base", &[]),
        ];
        let mut layout = Layout::new();
        let rows: Vec<GraphRow> = commits.iter().map(|c| layout.push(c)).collect();
        let text: Vec<String> = rows.iter().map(|row| row.text(false)).collect();
        assert_eq!(text, ["●", "│ ●", "● │", "│ ●", "│ ●", "●─╯"]);
        // Each branch keeps one colour from tip to fork point.
        let dev = rows[0].cells[0].color;
        let main = rows[1].cells[2].color;
        assert_ne!(dev, main);
        assert!(rows[1..5].iter().all(|row| row.cells[0].color == dev));
        assert!(rows[2..5].iter().all(|row| row.cells[2].color == main));
        assert_eq!(rows[5].cells[2].color, main);
    }

    #[test]
    fn a_merge_into_a_lane_that_continues_joins_it() {
        // `c` merges `q`, which `b`'s lane already waits for.
        let commits = [
            commit("a", &["p"]),
            commit("b", &["q"]),
            commit("c", &["r", "q"]),
        ];
        assert_eq!(graph(&commits), ["●", "│ ●", "│ ├─●"]);
    }

    #[test]
    fn a_line_crossing_a_lane_draws_a_cross() {
        // `p` merges `r`, two lanes to the right, past `mid`'s lane.
        let commits = [
            commit("left", &["p"]),
            commit("mid", &["q"]),
            commit("right", &["r"]),
            commit("p", &["z", "r"]),
        ];
        assert_eq!(graph(&commits)[3], "●─┼─┤");
    }

    #[test]
    fn several_children_converge_on_their_shared_parent() {
        let commits = [
            commit("c1", &["p"]),
            commit("c2", &["p"]),
            commit("c3", &["p"]),
            commit("p", &[]),
        ];
        assert_eq!(graph(&commits), ["●", "│ ●", "│ │ ●", "●─╯─╯"]);
    }

    #[test]
    fn a_new_tip_reuses_a_free_lane_and_ascii_stays_aligned() {
        let commits = [
            commit("m", &["a", "f"]),
            commit("f", &["a"]),
            commit("a", &[]),
            commit("other", &[]),
        ];
        let mut layout = Layout::new();
        let rows: Vec<GraphRow> = commits.iter().map(|c| layout.push(c)).collect();
        let ascii: Vec<String> = rows.iter().map(|row| row.text(true)).collect();
        assert_eq!(ascii, ["*-.", "| *", "*-'", "*"]);
        assert_eq!(rows[3].lane, 0);
    }

    #[test]
    fn the_layout_continues_across_pages() {
        let mut layout = Layout::new();
        let first: Vec<String> = [commit("m", &["b", "f"]), commit("f", &["a"])]
            .iter()
            .map(|c| layout.push(c).text(false))
            .collect();
        let second: Vec<String> = [commit("b", &["a"]), commit("a", &[])]
            .iter()
            .map(|c| layout.push(c).text(false))
            .collect();
        assert_eq!(first, ["●─╮", "│ ●"]);
        assert_eq!(second, ["● │", "●─╯"]);
    }
}
