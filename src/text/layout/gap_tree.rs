//! Gap-tree reading order, adapted from hiroi-sora/Umi-OCR (MIT).
//! See vendor/text-layout/SOURCE.md and LICENSE-Umi-OCR.txt.
//! Work with indices; text is never duplicated into tree nodes.
use super::Rect;

#[derive(Clone, Copy)]
struct Gap {
    left: f64,
    right: f64,
    first: usize,
}
#[derive(Clone, Copy)]
struct Cut {
    gap: Gap,
    last: usize,
}
struct Node {
    left: f64,
    right: f64,
    first: usize,
    last: usize,
    units: Vec<usize>,
    children: Vec<usize>,
}

/// Return column regions in reading order. Single-row gaps are not enough
/// evidence for a column: this keeps isolated labels and OCR fragments stable.
pub(super) fn regions(boxes: &[Rect]) -> Vec<Vec<usize>> {
    if boxes.len() < 4 || boxes.len() > 4096 {
        return vec![(0..boxes.len()).collect()];
    }
    let mut sorted: Vec<_> = (0..boxes.len()).collect();
    sorted.sort_by(|&a, &b| boxes[a].top.total_cmp(&boxes[b].top));
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for index in sorted {
        if let Some(row) = rows.last_mut().filter(|row| {
            let first = boxes[row[0]];
            let current = boxes[index];
            // Require actual baseline overlap, rather than a one-pixel touch.
            first.bottom.min(current.bottom) - first.top.max(current.top)
                >= first.height().min(current.height()) * 0.25
        }) {
            row.push(index);
        } else {
            rows.push(vec![index]);
        }
    }
    for row in &mut rows {
        row.sort_by(|&a, &b| boxes[a].left.total_cmp(&boxes[b].left));
    }
    let row_gutters: Vec<Vec<(f64, f64)>> = rows
        .iter()
        .map(|row| {
            row.windows(2)
                .filter_map(|pair| {
                    let a = boxes[pair[0]];
                    let b = boxes[pair[1]];
                    let height = a.height().min(b.height());
                    (b.left - a.right > height * 1.5 && a.width().min(b.width()) > height * 1.5)
                        .then_some((a.right, b.left))
                })
                .collect()
        })
        .collect();
    let repeated_gutter = row_gutters.windows(2).any(|pair| {
        pair[0]
            .iter()
            .any(|a| pair[1].iter().any(|b| a.1.min(b.1) > a.0.max(b.0)))
    });
    if !repeated_gutter {
        return vec![(0..boxes.len()).collect()];
    }
    let left = boxes.iter().map(|r| r.left).fold(f64::INFINITY, f64::min) - 1.;
    let right = boxes
        .iter()
        .map(|r| r.right)
        .fold(f64::NEG_INFINITY, f64::max)
        + 1.;
    let mut cuts = Vec::new();
    let mut active: Vec<Gap> = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        let mut gaps = Vec::new();
        let mut start = left;
        for &index in row {
            let rect = boxes[index];
            if rect.left > start {
                gaps.push(Gap {
                    left: start,
                    right: rect.left,
                    first: row_index,
                });
            }
            start = start.max(rect.right);
        }
        gaps.push(Gap {
            left: start,
            right,
            first: row_index,
        });
        let mut continued = vec![false; active.len()];
        let mut new_gap = vec![true; gaps.len()];
        let mut next = Vec::new();
        for (i, previous) in active.iter().enumerate() {
            for (j, current) in gaps.iter().enumerate() {
                let l = previous.left.max(current.left);
                let r = previous.right.min(current.right);
                if l <= r {
                    next.push(Gap {
                        left: l,
                        right: r,
                        first: previous.first,
                    });
                    continued[i] = true;
                    new_gap[j] = false;
                }
            }
        }
        next.extend(
            gaps.into_iter()
                .zip(new_gap)
                .filter_map(|(gap, new)| new.then_some(gap)),
        );
        for (gap, continued) in active.into_iter().zip(continued) {
            if !continued {
                cuts.push(Cut {
                    gap,
                    last: row_index - 1,
                });
            }
        }
        active = next;
    }
    cuts.extend(active.into_iter().map(|gap| Cut {
        gap,
        last: rows.len() - 1,
    }));
    cuts.sort_by(|a, b| a.gap.left.total_cmp(&b.gap.left));
    let mut row_gaps = vec![Vec::new(); rows.len()];
    for cut in cuts {
        for row in &mut row_gaps[cut.gap.first..=cut.last] {
            row.push((cut.gap.left, cut.gap.right));
        }
    }
    let mut nodes = vec![Node {
        left,
        right,
        first: 0,
        last: 0,
        units: Vec::new(),
        children: Vec::new(),
    }];
    let mut completed = vec![0];
    let mut pending: Vec<usize> = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        let gaps = &row_gaps[row_index];
        let mut next = Vec::new();
        for index in pending {
            let node = &nodes[index];
            let left_continues = gaps.iter().any(|gap| gap.1 == node.left);
            let right_continues = gaps.iter().any(|gap| gap.0 == node.right);
            let interrupted = gaps.iter().any(|gap| {
                (node.left < gap.0 && gap.0 < node.right)
                    || (node.left < gap.1 && gap.1 < node.right)
            });
            if left_continues && right_continues && !interrupted {
                nodes[index].last = row_index;
                next.push(index);
            } else {
                complete(index, &mut nodes, &mut completed);
            }
        }
        pending = next;
        for &unit in row {
            let x = boxes[unit].left;
            let Some(pair) = gaps
                .windows(2)
                .find(|pair| x + 0.0001 >= pair[0].1 && x < pair[1].0 + 0.0001)
            else {
                // Geometric degeneracy must not drop an OCR block.
                return vec![(0..boxes.len()).collect()];
            };
            let l = pair[0].1;
            let r = pair[1].0;
            if let Some(&index) = pending
                .iter()
                .find(|&&index| nodes[index].left == l && nodes[index].right == r)
            {
                nodes[index].units.push(unit);
            } else {
                let index = nodes.len();
                nodes.push(Node {
                    left: l,
                    right: r,
                    first: row_index,
                    last: row_index,
                    units: vec![unit],
                    children: Vec::new(),
                });
                pending.push(index);
            }
        }
    }
    for index in pending {
        complete(index, &mut nodes, &mut completed);
    }
    // Iterative traversal also bounds stack usage for long screenshots.
    for index in 0..nodes.len() {
        let mut children = std::mem::take(&mut nodes[index].children);
        children.sort_by(|&a, &b| nodes[a].left.total_cmp(&nodes[b].left));
        nodes[index].children = children;
    }
    let mut stack = vec![0];
    let mut output = Vec::new();
    while let Some(index) = stack.pop() {
        if !nodes[index].units.is_empty() {
            output.push(std::mem::take(&mut nodes[index].units));
        }
        stack.extend(nodes[index].children.iter().rev().copied());
    }
    if output.iter().map(Vec::len).sum::<usize>() != boxes.len() {
        return vec![(0..boxes.len()).collect()];
    }
    output
}

fn complete(index: usize, nodes: &mut [Node], completed: &mut Vec<usize>) {
    let node = &nodes[index];
    let x = (node.left + node.right) * 0.5;
    let parent = completed
        .iter()
        .copied()
        .filter(|&i| {
            i == 0 || (nodes[i].last < node.first && nodes[i].left <= x && x <= nodes[i].right)
        })
        .max_by(|&a, &b| {
            // The root is above every real row, including row zero.
            let level = |i: usize| if i == 0 { 0 } else { nodes[i].last + 1 };
            level(a)
                .cmp(&level(b))
                .then_with(|| nodes[a].right.total_cmp(&nodes[b].right))
        })
        .unwrap_or(0);
    nodes[parent].children.push(index);
    completed.push(index);
}
