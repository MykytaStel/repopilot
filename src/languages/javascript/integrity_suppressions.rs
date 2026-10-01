//! Lint, type, and coverage suppression directives in JS and TS comments.

use crate::review::signals::integrity::syntax::{Directive, comment_suppression};
use tree_sitter::Node;

const SUPPRESSIONS: &[Directive] = &[
    Directive {
        text: "@ts-ignore",
        takes_rules: false,
    },
    Directive {
        text: "@ts-expect-error",
        takes_rules: false,
    },
    Directive {
        text: "@ts-nocheck",
        takes_rules: false,
    },
    Directive {
        text: "eslint-disable-next-line",
        takes_rules: true,
    },
    Directive {
        text: "eslint-disable-line",
        takes_rules: true,
    },
    Directive {
        text: "eslint-disable",
        takes_rules: true,
    },
    Directive {
        text: "biome-ignore",
        takes_rules: true,
    },
    Directive {
        text: "tslint:disable",
        takes_rules: true,
    },
    Directive {
        text: "istanbul ignore",
        takes_rules: true,
    },
    Directive {
        text: "c8 ignore",
        takes_rules: true,
    },
];

pub(super) fn suppression<'a>(node: Node<'a>, content: &'a str) -> Option<String> {
    (node.kind() == "comment").then(|| comment_suppression(node, content, SUPPRESSIONS))?
}
