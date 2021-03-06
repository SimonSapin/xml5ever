use std::borrow::Cow::Borrowed;
use string_cache::QualName;
use tendril::StrTendril;
use tokenizer::{XTag, XPi};
use tree_builder::interface::{NodeOrText, TreeSink, AppendNode, AppendText};
use tree_builder::types::{XmlProcessResult, XDone};


pub trait XmlTreeBuilderActions<Handle> {
    fn current_node(&self) -> Handle;
    fn insert_appropriately(&mut self, child: NodeOrText<Handle>);
    fn insert_tag(&mut self, tag: XTag) -> XmlProcessResult;
    fn append_tag(&mut self, tag: XTag) -> XmlProcessResult;
    fn append_tag_to_doc(&mut self, tag: XTag) -> Handle;
    fn add_to_open_elems(&mut self, el: Handle) -> XmlProcessResult;
    fn append_comment_to_doc(&mut self, comment: StrTendril) -> XmlProcessResult;
    fn append_comment_to_tag(&mut self, text: StrTendril) -> XmlProcessResult;
    fn append_pi_to_doc(&mut self, pi: XPi) -> XmlProcessResult;
    fn append_pi_to_tag(&mut self, pi: XPi) -> XmlProcessResult;
    fn append_text(&mut self, chars: StrTendril) -> XmlProcessResult;
    fn tag_in_open_elems(&self, tag: &XTag) -> bool;
    fn pop_until<TagSet>(&mut self, pred: TagSet) where TagSet: Fn(QualName) -> bool;
    fn current_node_in<TagSet>(&self, set: TagSet) -> bool where TagSet: Fn(QualName) -> bool;
    fn close_tag(&mut self, tag: XTag) -> XmlProcessResult;
    fn no_open_elems(&self) -> bool;
    fn pop(&mut self) -> Handle ;
    fn stop_parsing(&mut self) -> XmlProcessResult;
}

#[doc(hidden)]
impl<Handle, Sink> XmlTreeBuilderActions<Handle>
    for super::XmlTreeBuilder<Handle, Sink>
    where Handle: Clone,
          Sink: TreeSink<Handle=Handle>,
{

    fn current_node(&self) -> Handle {
        self.open_elems.last().expect("no current element").clone()
    }

    fn insert_appropriately(&mut self, child: NodeOrText<Handle>){
        let target = self.current_node();
        self.sink.append(target, child);
    }

    fn insert_tag(&mut self, tag: XTag) -> XmlProcessResult {
        let child = self.sink.create_element(QualName::new(ns!(HTML),
            tag.name), tag.attrs);
        self.insert_appropriately(AppendNode(child.clone()));
        self.add_to_open_elems(child)
    }

    fn append_tag(&mut self, tag: XTag) -> XmlProcessResult {
        let child = self.sink.create_element(QualName::new(ns!(HTML),
            tag.name), tag.attrs);
        self.insert_appropriately(AppendNode(child));
        XDone
    }

    fn append_tag_to_doc(&mut self, tag: XTag) -> Handle {
        let root = self.doc_handle.clone();
        let child = self.sink.create_element(QualName::new(ns!(HTML),
            tag.name), tag.attrs);

        self.sink.append(root, AppendNode(child.clone()));
        child
    }

    fn add_to_open_elems(&mut self, el: Handle) -> XmlProcessResult {
        self.open_elems.push(el);

        //FIXME remove this on final commit
        println!("After add to open elems there are {} open elems", self.open_elems.len());
        XDone
    }

    fn append_comment_to_doc(&mut self, text: StrTendril) -> XmlProcessResult {
        let target = self.doc_handle.clone();
        let comment = self.sink.create_comment(text);
        self.sink.append(target, AppendNode(comment));
        XDone
    }

    fn append_comment_to_tag(&mut self, text: StrTendril) -> XmlProcessResult {
        let target = self.current_node();
        let comment = self.sink.create_comment(text);
        self.sink.append(target, AppendNode(comment));
        XDone
    }

    fn append_pi_to_doc(&mut self, pi: XPi) -> XmlProcessResult {
        let target = self.doc_handle.clone();
        let pi = self.sink.create_pi(pi.target, pi.data);
        self.sink.append(target, AppendNode(pi));
        XDone
    }

    fn append_pi_to_tag(&mut self, pi: XPi) -> XmlProcessResult {
        let target = self.current_node();
        let pi = self.sink.create_pi(pi.target, pi.data);
        self.sink.append(target, AppendNode(pi));
        XDone
    }


    fn append_text(&mut self, chars: StrTendril)
        -> XmlProcessResult {
        self.insert_appropriately(AppendText(chars));
        XDone
    }

    fn tag_in_open_elems(&self, tag: &XTag) -> bool {
        self.open_elems
            .iter()
            .any(|a| self.sink.elem_name(a) == QualName::new(ns!(HTML), tag.name.clone()))
    }

    // Pop elements until an element from the set has been popped.  Returns the
    // number of elements popped.
    fn pop_until<P>(&mut self, pred: P)
        where P: Fn(QualName) -> bool
    {
        loop {
            if self.current_node_in(|x| pred(x)) {
                break;
            }
            self.open_elems.pop();
        }
    }

    fn current_node_in<TagSet>(&self, set: TagSet) -> bool
        where TagSet: Fn(QualName) -> bool
    {
        set(self.sink.elem_name(&self.current_node()))
    }

    fn close_tag(&mut self, tag: XTag) -> XmlProcessResult {
        println!("Close tag: current_node.name {:?} \n Current tag {:?}",
                 self.sink.elem_name(&self.current_node()), &tag.name);
        if &self.sink.elem_name(&self.current_node()).local != &tag.name {
            self.sink.parse_error(Borrowed("Current node doesn't match tag"));
        }
        // FIXME remove this part after debug
        let is_closed = self.tag_in_open_elems(&tag);
        println!("Close tag {:?}", is_closed);

        if is_closed {
            // FIXME: Real namespace resolution
            self.pop_until(|p| p == QualName::new(ns!(HTML), tag.name.clone()));
            self.pop();
        }
        XDone
    }

    fn no_open_elems(&self) -> bool {
        self.open_elems.is_empty()
    }

    fn pop(&mut self) -> Handle {
        self.open_elems.pop().expect("no current element")
    }

    fn stop_parsing(&mut self) -> XmlProcessResult {
        warn!("stop_parsing for XML5 not implemented, full speed ahead!");
        XDone
    }
}
