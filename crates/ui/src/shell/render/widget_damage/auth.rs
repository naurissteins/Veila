use super::{WidgetDamage, WidgetRegions, merge, overlaps};

impl WidgetRegions {
    /// auth-only repainting is safe only when restoring its bounds preserves dynamic neighbors
    pub fn auth_damage_since(self, previous: Self) -> WidgetDamage {
        if self.size != previous.size || self.partial_unsafe || previous.partial_unsafe {
            return WidgetDamage::Full;
        }
        let neighbors = [
            (previous.header, self.header),
            (previous.media, self.media),
            (previous.indicators, self.indicators),
            (previous.weather, self.weather),
        ];
        if neighbors.iter().any(|(old, new)| old != new) {
            return WidgetDamage::Full;
        }
        // auth bounds already include paint padding and must cover disappearing status text
        let Some(damage) = merge(previous.auth, self.auth).filter(|rect| !rect.is_empty()) else {
            return WidgetDamage::Full;
        };
        // restoring the scene base would erase overlapping dynamic widgets
        if neighbors
            .iter()
            .filter_map(|(_, current)| *current)
            .any(|other| overlaps(damage, other.inflated(12)))
        {
            WidgetDamage::Full
        } else {
            WidgetDamage::Region(damage)
        }
    }
}

#[cfg(test)]
mod tests;
