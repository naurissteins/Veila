use std::{cell::RefCell, collections::VecDeque};

use crate::shell::{ShellState, ShellTheme};
use veila_renderer::RenderScale;

use super::TextLayoutCache;

const MAX_SCALED_CONTEXTS: usize = 4;

pub(in crate::shell) struct RenderContext<'a> {
    pub(in crate::shell) shell: &'a ShellState,
    pub(in crate::shell) theme: &'a ShellTheme,
    pub(in crate::shell) text_layout_cache: &'a RefCell<TextLayoutCache>,
    pub(in crate::shell) render_scale: RenderScale,
}

#[derive(Debug, Default)]
pub(in crate::shell) struct ScaledRenderCache {
    entries: VecDeque<ScaledRenderEntry>,
}

#[derive(Debug)]
struct ScaledRenderEntry {
    scale: RenderScale,
    theme: ShellTheme,
    text_layout_cache: RefCell<TextLayoutCache>,
}

impl ScaledRenderCache {
    pub(in crate::shell) fn clear(&mut self) {
        self.entries.clear();
    }

    fn get(&mut self, theme: &ShellTheme, scale: RenderScale) -> &ScaledRenderEntry {
        let index = match self.entries.iter().position(|entry| entry.scale == scale) {
            Some(index) => index,
            None => {
                if self.entries.len() == MAX_SCALED_CONTEXTS {
                    self.entries.pop_front();
                }
                self.entries.push_back(ScaledRenderEntry {
                    scale,
                    theme: theme.scaled_for_render_at(scale),
                    text_layout_cache: RefCell::default(),
                });
                self.entries.len() - 1
            }
        };
        &self.entries[index]
    }
}

impl ShellState {
    pub(in crate::shell) fn render_context(&self) -> RenderContext<'_> {
        RenderContext {
            shell: self,
            theme: &self.theme,
            text_layout_cache: &self.text_layout_cache,
            render_scale: RenderScale::ONE,
        }
    }

    pub(in crate::shell) fn emergency_render_context(&self, scale: u32) -> RenderContext<'_> {
        self.emergency_render_context_at(RenderScale::from_integer(scale))
    }

    pub(in crate::shell) fn emergency_render_context_at(
        &self,
        scale: RenderScale,
    ) -> RenderContext<'_> {
        RenderContext {
            render_scale: scale,
            ..self.render_context()
        }
    }

    pub(in crate::shell) fn with_render_scale<T>(
        &self,
        scale: u32,
        render: impl FnOnce(&RenderContext<'_>) -> T,
    ) -> T {
        self.with_pixel_scale(RenderScale::from_integer(scale), render)
    }

    pub(in crate::shell) fn with_pixel_scale<T>(
        &self,
        scale: RenderScale,
        render: impl FnOnce(&RenderContext<'_>) -> T,
    ) -> T {
        if scale == RenderScale::ONE || self.emergency_active() {
            return render(&self.emergency_render_context_at(scale));
        }
        let mut cache = self.scaled_render_cache.borrow_mut();
        let entry = cache.get(&self.theme, scale);
        render(&RenderContext {
            shell: self,
            theme: &entry.theme,
            text_layout_cache: &entry.text_layout_cache,
            render_scale: scale,
        })
    }
}

#[cfg(test)]
mod tests;
