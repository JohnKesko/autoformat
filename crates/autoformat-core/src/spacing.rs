//! Spacing rules shared by all languages.
//!
//! The rules only fix spacing that is clearly wrong: missing space after a comma, spaces
//! inside parentheses, missing spaces around `=`, and so on. When no rule applies, the
//! original spacing is kept. Language adapters handle their special cases first.

use crate::adapter::LanguageRules;
use crate::context::{Role, TokenContext};
use crate::layout::Spacing;

/// Decides the spacing between two tokens that end up on the same line.
pub fn common_spacing(previous: &TokenContext, next: &TokenContext, rules: &LanguageRules) -> Spacing {
    use Role::*;

    // Comments and unknown text keep their spacing.
    if matches!(previous.role, Comment | Gap) || matches!(next.role, Comment | Gap) {
        return Spacing::Keep;
    }

    // No space before `,` and `;`.
    if matches!(next.role, Comma | Semicolon) {
        return Spacing::NoSpace;
    }

    // Closing brackets: `(a)`, `[a]`, `{ a }`, `{}`.
    if next.role == Close {
        if next.is("}") && previous.role != Open {
            return Spacing::Space;
        }
        return Spacing::NoSpace;
    }

    // Opening brackets: `(a`, `[a`, `{ a`.
    if previous.role == Open {
        if previous.is("{") {
            return Spacing::Space;
        }
        return Spacing::NoSpace;
    }

    // `a.b`, `a::b`, `a?.b`, `0..10`. After a comma or operator, `..` starts an operand.
    if next.role == Tight && !matches!(previous.role, Comma | Semicolon | Infix | Colon) {
        if previous.role == Keyword {
            return Spacing::Keep;
        }
        return Spacing::NoSpace;
    }
    if previous.role == Tight {
        return Spacing::NoSpace;
    }

    // `name: value`
    if next.role == Colon {
        return Spacing::NoSpace;
    }
    if previous.role == Colon {
        return Spacing::Space;
    }

    // `=>` is always spaced: `x => x + 1`, `Some(x) => x`, `int Get() => 1;`.
    if next.is("=>") || previous.is("=>") {
        return Spacing::Space;
    }

    // `x?`, `x++`, `<T,>`. Next to a keyword the source spacing is kept.
    if next.role == Postfix {
        if previous.role == Keyword {
            return Spacing::Keep;
        }
        return Spacing::NoSpace;
    }

    // `a, b` and `a; b`
    if matches!(previous.role, Comma | Semicolon) {
        return Spacing::Space;
    }

    // `-x`, `!x`. Next to a keyword the source spacing is kept (`import * as name`).
    if previous.role == Prefix {
        if next.role == Keyword {
            return Spacing::Keep;
        }
        return Spacing::NoSpace;
    }

    // `a + b`, `x = 1`, `a => b`
    if next.role == Infix || previous.role == Infix {
        return Spacing::Space;
    }

    if next.role == Open && next.is("(") {
        // `if (x)`
        if previous.role == Keyword && rules.space_before_paren_keywords.contains(&previous.text) {
            return Spacing::Space;
        }
        // `call(x)`, `f()()`, `generic<T>(x)`
        let previous_is_callee = previous.role == Word
            || (previous.role == Postfix && previous.is(">"))
            || (previous.role == Close && !previous.is("}"));
        if previous_is_callee {
            return Spacing::NoSpace;
        }
        return Spacing::Keep;
    }

    // `items[0]`, `string[]`
    if next.role == Open && next.is("[") {
        if rules.index_kinds.contains(&next.parent_kind) {
            return Spacing::NoSpace;
        }
        return Spacing::Keep;
    }

    // `) {`, `else {`, `x = {`
    if next.role == Open && next.is("{") {
        return Spacing::Space;
    }

    // `} else`
    if previous.role == Close && previous.is("}") {
        return Spacing::Space;
    }

    Spacing::Keep
}
