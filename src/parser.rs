// -------------------------------------------------------------------------------------------------
// AST node.

#[derive(Clone, Debug, PartialEq)]
pub enum AstNode {
    Literal(AstValue),
    Identifier(String),
    Call(String, Vec<AstNode>),
    Assign(String, Box<AstNode>),
    If {
        cond_expr: Box<AstNode>,
        true_expr: Vec<AstNode>,
        false_expr: Vec<AstNode>,
    },
    For {
        ident: String,
        first: i64,
        last: i64,
        body: Vec<AstNode>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum AstValue {
    Int(i64),
    Text(String),
}

// -------------------------------------------------------------------------------------------------

pub fn parse_string(input: &str) -> Result<AstNode, std::io::Error> {
    Ok(fbl_parser::parse(input)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?)
}

// -------------------------------------------------------------------------------------------------

peg::parser! {
    grammar fbl_parser() for str {
        pub rule parse() -> AstNode
            = s:stmt() eoi() {
                s
            }

        rule stmt() -> AstNode
            = for_loop_stmt()
            / if_stmt()
            / assign_stmt()
            / e:expr() ";" _ { e }

        rule stmt_list() -> Vec<AstNode>
            = ss:stmt()* {
                ss
            }

        rule for_loop_stmt() -> AstNode
            = "for" _ "(" _ id:ident() ";" _ fst:num() "," _ lst:num() ")" _ "{" _
                  b:stmt_list()
              "}" _ {
                AstNode::For {
                    ident: id,
                    first: fst,
                    last: lst,
                    body: b,
                }
            }

        rule if_stmt() -> AstNode
            = "if" _  "(" _ ce:expr() ")" _ "{" _
                ts:stmt_list()
            "}" _ "else" _ "{" _
                fs:stmt_list()
            "}" _ {
                AstNode::If {
                    cond_expr: Box::new(ce),
                    true_expr: ts,
                    false_expr: fs,
                }
            }

        rule assign_stmt() -> AstNode
            = i:ident() "=" _ e:expr() ";" _ {
                AstNode::Assign(i, Box::new(e))
            }

        rule expr() -> AstNode
            = precedence! {
                l:(@) "&&" _ r:@ { AstNode::Call("&&".to_string(), vec![l, r]) }
                --
                l:(@) "==" _ r:@ { AstNode::Call("==".to_string(), vec![l, r]) }
                --
                l:(@) "%" _ r:@ { AstNode::Call("%".to_string(), vec![l, r]) }
                --
                t:term() { t }
            }

        rule term() -> AstNode
            = call_expr()
            / i:ident() { AstNode::Identifier(i) }
            / l:literal() { AstNode::Literal(l) }
            / "(" _ e:expr() ")" _ { e }

        rule call_expr() -> AstNode
            = i:ident() "(" _ args:(expr() ** ("," _)) ")" _ {
                AstNode::Call(i, args)
            }

        rule ident() -> String
            = !keyword() id:$(id_char0() id_char()*) _ {
                id.to_string()
            }

        rule id_char0()
            = ['A'..='Z' | 'a'..='z' | '_']

        rule id_char()
            = id_char0() / ['0'..='9']

        rule keyword()
            = "for" / "if" / "else"

        rule literal() -> AstValue
            = n:num() {
                AstValue::Int(n)
            }
            / "\"" s:$((!"\"" [_])*) "\"" _ {
                AstValue::Text(s.to_string())
            }

        rule num() -> i64
            = i:$(['0'..='9']+) _ {
                i.parse::<i64>().unwrap()
            }

        rule _()
            = quiet!{ws() / comment()}*

        rule comment()
            = "//" (!['\n' | '\r'] [_])*

        rule ws()
            = [' ' | '\n' | '\r' | '\t' ]

        rule eoi()
            = ![_] / expected!("end of input")

    }
}

// -------------------------------------------------------------------------------------------------