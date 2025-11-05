# GFM Feature Showcase

## Table With Alignment

| Left | Center | Right |
| :--- | :----: | ----: |
| Alpha | Beta | Gamma |
| Longer cell | Inline `code` | ~~Struck~~ |

## Task List

- [x] Checked item
- [ ] Pending item

## Alerts

> [!NOTE]
> This is a GitHub alert with a default title.

> [!TIP]
> Remember to hydrate during long writing sessions.

> [!IMPORTANT]
> Releases happen every Friday at 17:00 UTC.

> [!WARNING]
> Preview links expire after 24 hours.

> [!CAUTION]
> Editing production config in the browser can break live traffic.

## Description List

Feature
: Describes the thing using definition list syntax.

## Footnote

Footnote reference.[^ref]

[^ref]: Footnote body with a [link](https://example.com).

## Inline Styles

~~Strikethrough~~, ++Underline++, ==Highlight==, and regular text.

## Code Block

```rust
fn main() {
    println!("Hello, world!");
}
```

## Autolink

Visit https://github.com for more details.

## Autolink Literals

Visit www.github.com for more info or email support@example.com for help.

## HTML Filtering

<title>Leaked Title</title>

<script>alert('inline script should be stripped');</script>

## HTML Preservation

Allow inline edits like <ins>inserted text</ins> to pass through when safe.
