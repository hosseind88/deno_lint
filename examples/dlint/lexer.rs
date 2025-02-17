// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.

//! This module is mostly brought from https://github.com/denoland/deno/blob/96d05829002ef065b8fc84fe70de062cff0e95b3/cli/ast/mod.rs

use deno_ast::swc::ast::EsVersion;
use deno_ast::swc::common::comments::{
  Comment, CommentKind, SingleThreadedComments,
};
use deno_ast::swc::common::{FileName, SourceMap, Span};
use deno_ast::swc::parser::lexer::Lexer;
use deno_ast::swc::parser::token::Token;
use deno_ast::swc::parser::{EsConfig, StringInput, Syntax, TsConfig};
use std::convert::TryFrom;
use std::ops::Range;

static ES_VERSION: EsVersion = EsVersion::Es2021;

pub fn lex(source: &str, media_type: MediaType) -> Vec<LexedItem> {
  let source_map = SourceMap::default();
  let source_file = source_map.new_source_file(
    FileName::Custom(format!("anonymous.{}", media_type.ext())),
    source.to_string(),
  );
  let comments = SingleThreadedComments::default();
  let lexer = Lexer::new(
    media_type.syntax(),
    ES_VERSION,
    StringInput::from(source_file.as_ref()),
    Some(&comments),
  );

  let mut tokens: Vec<LexedItem> = lexer
    .map(|token| LexedItem {
      span: token.span,
      inner: TokenOrComment::Token(token.token),
    })
    .collect();

  tokens.extend(flatten_comments(comments).map(|comment| LexedItem {
    span: comment.span,
    inner: TokenOrComment::Comment {
      kind: comment.kind,
      text: comment.text,
    },
  }));

  tokens.sort_by_key(|item| item.span.lo.0);

  tokens
}

#[derive(Debug, Clone, Copy)]
pub enum MediaType {
  JavaScript,
  TypeScript,
  Jsx,
  Tsx,
  Dts,
}

impl TryFrom<&str> for MediaType {
  type Error = ();

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    match value {
      "javascript" => Ok(Self::JavaScript),
      "typescript" => Ok(Self::TypeScript),
      "jsx" => Ok(Self::Jsx),
      "tsx" => Ok(Self::Tsx),
      "dts" => Ok(Self::Dts),
      _ => Err(()),
    }
  }
}

impl MediaType {
  fn ext(&self) -> &'static str {
    use MediaType::*;
    match *self {
      JavaScript => "js",
      TypeScript => "ts",
      Jsx => "jsx",
      Tsx => "tsx",
      Dts => "d.ts",
    }
  }

  fn syntax(&self) -> Syntax {
    fn get_es_config(jsx: bool) -> EsConfig {
      EsConfig {
        export_default_from: true,
        jsx,
        ..EsConfig::default()
      }
    }
    fn get_ts_config(tsx: bool, dts: bool) -> TsConfig {
      TsConfig {
        decorators: true,
        dts,
        tsx,
        ..TsConfig::default()
      }
    }

    use MediaType::*;
    match *self {
      JavaScript => Syntax::Es(get_es_config(false)),
      TypeScript => Syntax::Typescript(get_ts_config(false, false)),
      Jsx => Syntax::Es(get_es_config(true)),
      Tsx => Syntax::Typescript(get_ts_config(true, false)),
      Dts => Syntax::Typescript(get_ts_config(false, true)),
    }
  }
}

#[derive(Debug)]
pub enum TokenOrComment {
  Token(Token),
  Comment { kind: CommentKind, text: String },
}

#[derive(Debug)]
pub struct LexedItem {
  pub span: Span,
  pub inner: TokenOrComment,
}

impl LexedItem {
  pub fn span_as_range(&self) -> Range<usize> {
    self.span.lo.0 as usize..self.span.hi.0 as usize
  }
}

fn flatten_comments(
  comments: SingleThreadedComments,
) -> impl Iterator<Item = Comment> {
  let (leading, trailing) = comments.take_all();
  let mut comments = (*leading).clone().into_inner();
  comments.extend((*trailing).clone().into_inner());
  comments.into_iter().flat_map(|el| el.1)
}
