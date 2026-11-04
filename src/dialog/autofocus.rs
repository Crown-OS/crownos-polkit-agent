use crownui::{
    ext::{Kind, NodeId},
    prelude::{Cx, Element, View},
};

/// Wraps a view whose root takes keyboard focus as soon as it mounts.
pub struct AutofocusKind<V> {
    content: V,
}

impl<V: View + 'static> Kind for AutofocusKind<V> {
    const NAME: &'static str = "autofocus";

    fn mount(self, cx: &mut Cx, node: NodeId) {
        let content = cx.build(self.content);
        cx.adopt_children(node, vec![content]);
        cx.request_focus(content);
    }
}

pub fn autofocus<V: View + 'static>(content: V) -> Element<AutofocusKind<V>> {
    Element::new(AutofocusKind { content })
}
