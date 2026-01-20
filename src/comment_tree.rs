//! Comment tree management for nested HN comment threads.
//!
//! Handles expansion state and visibility calculation for a flat list of comments
//! with depth information.

use std::collections::HashSet;

use crate::api::Comment;

/// Manages a comment tree's expansion state and visibility.
///
/// Comments are stored as a flat list with depth information. The `CommentTree`
/// tracks which comments are expanded and computes which comments should be visible
/// based on their ancestors' expansion state.
#[derive(Debug, Default)]
pub struct CommentTree {
    comments: Vec<Comment>,
    expanded: HashSet<u64>,
}

impl CommentTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the comment list and expand all comments.
    pub fn set(&mut self, comments: Vec<Comment>) {
        self.comments = comments;
        self.expand_all();
    }

    /// Clear all comments and expansion state.
    pub fn clear(&mut self) {
        self.comments.clear();
        self.expanded.clear();
    }

    /// Get the underlying comments slice.
    pub fn comments(&self) -> &[Comment] {
        &self.comments
    }

    /// Get a comment by its actual index in the flat list.
    pub fn get(&self, index: usize) -> Option<&Comment> {
        self.comments.get(index)
    }

    /// Get a mutable reference to a comment by its id.
    pub fn get_mut(&mut self, id: u64) -> Option<&mut Comment> {
        self.comments.iter_mut().find(|c| c.id == id)
    }

    /// Check if a comment is expanded.
    pub fn is_expanded(&self, id: u64) -> bool {
        self.expanded.contains(&id)
    }

    /// Check if the tree is empty.
    pub const fn is_empty(&self) -> bool {
        self.comments.is_empty()
    }

    /// Total number of comments (not just visible).
    pub const fn len(&self) -> usize {
        self.comments.len()
    }

    /// Compute indices of visible comments based on expansion state.
    ///
    /// A comment is visible if all its ancestors are expanded.
    pub fn visible_indices(&self) -> Vec<usize> {
        let mut visible = Vec::new();
        let mut parent_visible_at_depth: Vec<bool> = vec![true];

        for (i, comment) in self.comments.iter().enumerate() {
            parent_visible_at_depth.truncate(comment.depth + 1);

            let is_visible = parent_visible_at_depth
                .get(comment.depth)
                .copied()
                .unwrap_or(false);

            if is_visible {
                visible.push(i);

                let children_visible = self.expanded.contains(&comment.id);
                if parent_visible_at_depth.len() <= comment.depth + 1 {
                    parent_visible_at_depth.push(children_visible);
                } else {
                    parent_visible_at_depth[comment.depth + 1] = children_visible;
                }
            }
        }

        visible
    }

    /// Number of currently visible comments.
    pub fn visible_count(&self) -> usize {
        self.visible_indices().len()
    }

    /// Expand a comment by ID. Returns true if it was newly expanded.
    pub fn expand(&mut self, id: u64) -> bool {
        self.expanded.insert(id)
    }

    /// Collapse a comment by ID. Returns true if it was previously expanded.
    pub fn collapse(&mut self, id: u64) -> bool {
        self.expanded.remove(&id)
    }

    /// Expand a comment and all its descendants.
    ///
    /// `start_index` is the actual index in the flat comment list.
    pub fn expand_subtree(&mut self, start_index: usize) {
        let Some(start_comment) = self.comments.get(start_index) else {
            return;
        };
        let start_depth = start_comment.depth;

        for i in start_index..self.comments.len() {
            let comment = &self.comments[i];
            // Stop when we reach a comment at the same or higher level (not a descendant)
            if i > start_index && comment.depth <= start_depth {
                break;
            }
            if !comment.kids.is_empty() {
                self.expanded.insert(comment.id);
            }
        }
    }

    /// Collapse a comment and all its descendants.
    ///
    /// `start_index` is the actual index in the flat comment list.
    pub fn collapse_subtree(&mut self, start_index: usize) {
        let Some(start_comment) = self.comments.get(start_index) else {
            return;
        };
        let start_depth = start_comment.depth;

        for i in start_index..self.comments.len() {
            let comment = &self.comments[i];
            if i > start_index && comment.depth <= start_depth {
                break;
            }
            self.expanded.remove(&comment.id);
        }
    }

    /// Expand all comments that have children.
    pub fn expand_all(&mut self) {
        for comment in &self.comments {
            if !comment.kids.is_empty() {
                self.expanded.insert(comment.id);
            }
        }
    }

    /// Collapse all comments.
    pub fn collapse_all(&mut self) {
        self.expanded.clear();
    }

    /// Find the actual index of the top-level ancestor for a comment.
    ///
    /// Given a visible index, walks backward through visible comments to find
    /// the first depth-0 comment.
    pub fn find_toplevel_ancestor(
        &self,
        visible_indices: &[usize],
        visible_index: usize,
    ) -> Option<(usize, usize)> {
        let actual_idx = visible_indices.get(visible_index).copied()?;
        let comment = self.comments.get(actual_idx)?;

        if comment.depth == 0 {
            return Some((visible_index, actual_idx));
        }

        for i in (0..visible_index).rev() {
            if let Some(&actual) = visible_indices.get(i)
                && self.comments[actual].depth == 0
            {
                return Some((i, actual));
            }
        }

        None
    }

    /// Find the visible index of the parent comment.
    ///
    /// Walks backward through visible comments to find a comment at a lower depth.
    pub fn find_parent_visible_index(
        &self,
        visible_indices: &[usize],
        visible_index: usize,
    ) -> Option<usize> {
        let actual_idx = visible_indices.get(visible_index).copied()?;
        let current_depth = self.comments.get(actual_idx)?.depth;

        if current_depth == 0 {
            return None;
        }

        for i in (0..visible_index).rev() {
            if let Some(&actual) = visible_indices.get(i)
                && self.comments[actual].depth < current_depth
            {
                return Some(i);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::CommentBuilder;

    fn sample_tree() -> Vec<Comment> {
        vec![
            CommentBuilder::new()
                .id(1)
                .depth(0)
                .kids(vec![2, 3])
                .build(),
            CommentBuilder::new().id(2).depth(1).kids(vec![4]).build(),
            CommentBuilder::new().id(4).depth(2).build(),
            CommentBuilder::new().id(3).depth(1).build(),
            CommentBuilder::new().id(5).depth(0).kids(vec![6]).build(),
            CommentBuilder::new().id(6).depth(1).build(),
        ]
    }

    #[test]
    fn test_new_tree_is_empty() {
        let tree = CommentTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
    }

    #[test]
    fn test_set_comments() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());

        assert!(!tree.is_empty());
        assert_eq!(tree.len(), 6);
    }

    #[test]
    fn test_clear() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.expand(1);
        tree.clear();

        assert!(tree.is_empty());
        assert!(!tree.is_expanded(1));
    }

    #[test]
    fn test_visible_indices_all_collapsed() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.collapse_all();

        // Only top-level comments visible when all collapsed
        let visible = tree.visible_indices();
        assert_eq!(visible, vec![0, 4]); // Comments 1 and 5 (indices 0 and 4)
    }

    #[test]
    fn test_visible_indices_with_expansion() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.collapse_all();
        tree.expand(1); // Expand first comment

        let visible = tree.visible_indices();
        // Comment 1 (idx 0) expanded shows children 2 (idx 1) and 3 (idx 3)
        // Comment 5 (idx 4) still visible
        assert_eq!(visible, vec![0, 1, 3, 4]);
    }

    #[test]
    fn test_visible_indices_deep_expansion() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.collapse_all();
        tree.expand(1); // Expand comment 1
        tree.expand(2); // Expand comment 2

        let visible = tree.visible_indices();
        // Now comment 4 (idx 2) is also visible
        assert_eq!(visible, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_expand_collapse() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.collapse_all();

        assert!(!tree.is_expanded(1));
        assert!(tree.expand(1));
        assert!(tree.is_expanded(1));
        assert!(!tree.expand(1)); // Already expanded

        assert!(tree.collapse(1));
        assert!(!tree.is_expanded(1));
        assert!(!tree.collapse(1)); // Already collapsed
    }

    #[test]
    fn test_expand_subtree() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.collapse_all();
        tree.expand_subtree(0); // Expand comment 1 and descendants

        assert!(tree.is_expanded(1));
        assert!(tree.is_expanded(2));
        // Comment 4 has no kids, so not in expanded set (but that's fine)
        assert!(!tree.is_expanded(5)); // Comment 5 not affected
    }

    #[test]
    fn test_collapse_subtree() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.expand_all();
        tree.collapse_subtree(0); // Collapse comment 1 and descendants

        assert!(!tree.is_expanded(1));
        assert!(!tree.is_expanded(2));
        assert!(tree.is_expanded(5)); // Comment 5 not affected
    }

    #[test]
    fn test_expand_all() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.expand_all();

        assert!(tree.is_expanded(1));
        assert!(tree.is_expanded(2));
        assert!(tree.is_expanded(5));
        // Comments without kids (3, 4, 6) aren't in expanded set
    }

    #[test]
    fn test_collapse_all() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.expand_all();
        tree.collapse_all();

        assert!(!tree.is_expanded(1));
        assert!(!tree.is_expanded(2));
        assert!(!tree.is_expanded(5));
    }

    #[test]
    fn test_find_toplevel_ancestor_at_root() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.expand(1);

        let visible = tree.visible_indices();
        let result = tree.find_toplevel_ancestor(&visible, 0);
        assert_eq!(result, Some((0, 0))); // Already at top-level
    }

    #[test]
    fn test_find_toplevel_ancestor_nested() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.expand(1);
        tree.expand(2);

        let visible = tree.visible_indices();
        // visible = [0, 1, 2, 3, 4]
        // Comment at visible index 2 (actual 2, depth 2) -> ancestor at visible 0
        let result = tree.find_toplevel_ancestor(&visible, 2);
        assert_eq!(result, Some((0, 0)));
    }

    #[test]
    fn test_find_parent_visible_index() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());
        tree.expand(1);
        tree.expand(2);

        let visible = tree.visible_indices();
        // visible = [0, 1, 2, 3, 4]
        // Comment at visible 2 (actual 2, depth 2) -> parent at visible 1 (depth 1)
        let result = tree.find_parent_visible_index(&visible, 2);
        assert_eq!(result, Some(1));

        // Comment at visible 1 (actual 1, depth 1) -> parent at visible 0 (depth 0)
        let result = tree.find_parent_visible_index(&visible, 1);
        assert_eq!(result, Some(0));

        // Comment at visible 0 (depth 0) -> no parent
        let result = tree.find_parent_visible_index(&visible, 0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_visible_count() {
        let mut tree = CommentTree::new();
        tree.set(sample_tree());

        assert_eq!(tree.visible_count(), 6); // All expanded by default

        tree.collapse_all();
        assert_eq!(tree.visible_count(), 2); // Only top-level

        tree.expand(1);
        assert_eq!(tree.visible_count(), 4); // +2 children of comment 1
    }
}
