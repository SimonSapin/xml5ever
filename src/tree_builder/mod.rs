mod actions;
mod rules;
mod types;
// "pub" is a workaround for rust#18241 (?)
pub mod interface;

pub use self::interface::{TreeSink, Tracer, NextParserState, NodeOrText};

use self::rules::XmlTreeBuilderStep;
use self::types::*;
use std::collections::VecDeque;
use tokenizer::{self, XTokenSink};

// The XML tree builder.
pub struct XmlTreeBuilder<Handle, Sink> {
    /// Consumer of tree modifications.
    sink: Sink,

    /// The document node, which is created by the sink.
    doc_handle: Handle,

    /// Next state change for the tokenizer, if any.
    next_tokenizer_state: Option<tokenizer::states::XmlState>,

    /// Stack of open elements, most recently added at end.
    open_elems: Vec<Handle>,

    /// Current element pointer.
    curr_elem: Option<Handle>,

    /// Current tree builder phase.
    phase: XmlPhase,
}
impl<Handle, Sink> XmlTreeBuilder<Handle, Sink>
    where Handle: Clone,
          Sink: TreeSink<Handle=Handle>,
{
    /// Create a new tree builder which sends tree modifications to a particular `TreeSink`.
    ///
    /// The tree builder is also a `TokenSink`.
    pub fn new(mut sink: Sink) -> XmlTreeBuilder<Handle, Sink> {
        let doc_handle = sink.get_document();
        XmlTreeBuilder {
            sink: sink,
            doc_handle: doc_handle,
            next_tokenizer_state: None,
            open_elems: vec!(),
            curr_elem: None,
            phase: StartPhase,
        }
    }

    pub fn unwrap(self) -> Sink {
        self.sink
    }

    pub fn sink<'a>(&'a self) -> &'a Sink {
        &self.sink
    }

    pub fn sink_mut<'a>(&'a mut self) -> &'a mut Sink {
        &mut self.sink
    }

    /// Call the `Tracer`'s `trace_handle` method on every `Handle` in the tree builder's
    /// internal state.  This is intended to support garbage-collected DOMs.
    pub fn trace_handles(&self, tracer: &Tracer<Handle=Handle>) {
        tracer.trace_handle(self.doc_handle.clone());
        for e in self.open_elems.iter() {
            tracer.trace_handle(e.clone());
        }
        self.curr_elem.as_ref().map(|h| tracer.trace_handle(h.clone()));
    }

    // Debug helper
    #[cfg(not(for_c))]
    #[allow(dead_code)]
    fn dump_state(&self, label: String) {
        use string_cache::QualName;

        println!("dump_state on {}", label);
        print!("    open_elems:");
        for node in self.open_elems.iter() {
            let QualName { ns, local } = self.sink.elem_name(node);
            print!(" {:?}:{:?}", ns,local);

        }
        println!("");
    }

    #[cfg(for_c)]
    fn debug_step(&self, _mode: XmlPhase, _token: &XToken) {
    }

    #[cfg(not(for_c))]
    fn debug_step(&self, mode: XmlPhase, token: &XToken) {
        debug!("processing {:?} in insertion mode {:?}", format!("{:?}", token), mode);
    }

    fn process_to_completion(&mut self, mut token: XToken) {
        // Queue of additional tokens yet to be processed.
        // This stays empty in the common case where we don't split whitespace.
        let mut more_tokens = VecDeque::new();

        loop {
            let phase = self.phase;
            match self.step(phase, token) {
                XDone => {
                    token = unwrap_or_return!(more_tokens.pop_front(), ());
                }
                XReprocess(m, t) => {
                    self.phase = m;
                    token = t;
                }

            }
        }
    }
}

impl<Handle, Sink> XTokenSink
    for XmlTreeBuilder<Handle, Sink>
    where Handle: Clone,
          Sink: TreeSink<Handle=Handle>,
{
    fn process_token(&mut self, token: tokenizer::XToken) {
        //let ignore_lf = replace(&mut self.ignore_lf, false);

        // Handle `ParseError` and `DoctypeToken`; convert everything else to the local `Token` type.
        let token = match token {
            tokenizer::XParseError(e) => {
                self.sink.parse_error(e);
                return;
            }

            tokenizer::DoctypeXToken(_) => {
                panic!("Doctype not implemented!!");
            }

            tokenizer::PIToken(x)   => PIToken(x),
            tokenizer::XTagToken(x) => XTagToken(x),
            tokenizer::CommentXToken(x) => CommentXToken(x),
            tokenizer::NullCharacterXToken => NullCharacterXToken,
            tokenizer::EOFXToken => EOFXToken,
            tokenizer::CharacterXTokens(x) => CharacterXTokens(x),

        };

        self.process_to_completion(token);
    }

    fn query_state_change(&mut self) -> Option<tokenizer::states::XmlState> {
        self.next_tokenizer_state.take()
    }
}
