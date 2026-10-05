//! Renamed tests: a rename that keeps part of the old body but drops a check,
//! and a rename that now checks other properties.

use super::IntegrityKind;
use super::tests::{detect, kinds};

/// A renamed test that keeps its setup but now checks other properties is a
/// substitution, even with as many assertions (po-ui/po-angular, popover
/// width: `width` and the position call became `visibility` and `left`).
#[test]
fn a_renamed_test_checking_other_properties_is_still_reported() {
    let before = r#"
describe("PoPopoverComponent", () => {
  it("open: should set widthPopover and call requestAnimationFrame when cornerAligned is true", () => {
    const fakeNativeElement = { style: { width: '', opacity: 0 }, scrollWidth: 250 };
    const fakeThis = { cornerAligned: true, popoverElement: { nativeElement: fakeNativeElement }, setPopoverPosition: () => {} };
    spyOn(fakeThis, 'setPopoverPosition');
    component.open.call(fakeThis);
    expect(fakeNativeElement.style.width).toBe('auto');
    expect(fakeThis.setPopoverPosition).toHaveBeenCalled();
  });
});
"#;
    let after = r#"
describe("PoPopoverComponent", () => {
  it("open: should set widthPopover from getBoundingClientRect when cornerAligned is true", () => {
    const fakeNativeElement = { style: { width: '', opacity: 0, visibility: '', left: '' }, scrollWidth: 250 };
    const fakeThis = { cornerAligned: true, popoverElement: { nativeElement: fakeNativeElement }, setPopoverPosition: () => {} };
    spyOn(fakeThis, 'setPopoverPosition');
    component.open.call(fakeThis);
    expect(fakeNativeElement.style.visibility).toBe('');
    expect(fakeNativeElement.style.left).toBe('');
  });
});
"#;
    let signals = detect(
        "src/po-popover.component.spec.ts",
        "TypeScript",
        Some(before),
        Some(after),
    );
    assert_eq!(
        kinds(&signals),
        vec![IntegrityKind::TestRemoved],
        "{signals:?}"
    );
}

/// A renamed test that keeps part of its old body and drops a check is an
/// assertion drop, not a removed test (rolling-scopes/rsschool-app#2989).
#[test]
fn a_renamed_test_reduced_to_part_of_its_body_reports_the_dropped_check() {
    let before = r#"
describe("MentorCard", () => {
  it('renders "Say Thank you!" button with link to /gratitude', () => {
    const link = screen.getByRole('link', { name: /say thank you/i });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute('href', '/gratitude?githubId=testmentor');
  });
});
"#;
    let after = r#"
describe("MentorCard", () => {
  it('renders "Say Thank you!" button that navigates to /gratitude', () => {
    const button = screen.getByRole('button', { name: /say thank you/i });
    expect(button).toBeInTheDocument();
  });
});
"#;
    let signals = detect(
        "src/MentorCard.test.tsx",
        "TypeScript",
        Some(before),
        Some(after),
    );
    assert_eq!(
        kinds(&signals),
        vec![IntegrityKind::AssertionsRemoved],
        "{signals:?}"
    );
    assert_eq!(
        signals[0].detail,
        "\"MentorCard > renders \"Say Thank you!\" button with link to /gratitude\" renamed to \
         \"MentorCard > renders \"Say Thank you!\" button that navigates to /gratitude\": assertions 2 → 1"
    );
}

/// A new test that shares a few calls with a removed one but checks as much
/// is a different test: the removal stays reported.
#[test]
fn a_new_test_with_a_shared_setup_does_not_hide_a_removed_test() {
    let before = r#"
describe("cart", () => {
  it("rejects negative quantities", () => {
    const cart = render(Cart);
    expect(() => cart.add(item(1, -1))).toThrow();
  });
});
"#;
    let after = r#"
describe("cart", () => {
  it("shows the empty state", () => {
    const cart = render(Cart);
    expect(cart.text()).toContain("empty");
  });
});
"#;
    let signals = detect("src/cart.test.ts", "TypeScript", Some(before), Some(after));
    assert_eq!(
        kinds(&signals),
        vec![IntegrityKind::TestRemoved],
        "{signals:?}"
    );
}
