use std::char;
use pest::Parser;
use pest::iterators::{Pairs, Pair};
use crate::parser::ast::{GuardRule, RuleLevel, RuleScope, Expr, Operator, RuleAssert};

pub mod ast;

#[derive(Parser)]
#[grammar = "parser/guarding.pest"]
struct IdentParser;

pub fn parse(code: &str) -> Vec<GuardRule> {
    let pairs = IdentParser::parse(Rule::start, code).unwrap_or_else(|e| panic!("{}", e));
    consume_rules_with_spans(pairs)
}

fn consume_rules_with_spans(pairs: Pairs<Rule>) -> Vec<GuardRule> {
    pairs.filter(|pair| {
        return pair.as_rule() == Rule::declaration;
    }).map(|pair| {
        let mut rule: GuardRule = Default::default();
        for p in pair.into_inner() {
            match p.as_rule() {
                Rule::normal_rule => {
                    rule = parse_normal_rule(p);
                }
                Rule::layer_rule => {
                    rule = GuardRule::default();
                }
                _ => panic!("unreachable content rule: {:?}", p.as_rule())
            };
        }

        return rule;
    })
        .collect::<Vec<GuardRule>>()
}

fn parse_normal_rule(pair: Pair<Rule>) -> GuardRule {
    let mut guard_rule = GuardRule::default();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::rule_level => {
                let level = p.as_span().as_str();
                match level {
                    "module" => { guard_rule.level = RuleLevel::Module }
                    "package" => { guard_rule.level = RuleLevel::Package }
                    "function" => { guard_rule.level = RuleLevel::Function }
                    "file" => { guard_rule.level = RuleLevel::File }
                    "class" => { guard_rule.level = RuleLevel::Class }
                    &_ => { unreachable!("error rule level: {:?}", level) }
                };
            }
            Rule::use_symbol => {
                // may be can do something, but still nothing.
            }
            Rule::expression => {
                guard_rule.expr = parse_expr(p);
            }
            Rule::operator => {
                guard_rule.ops = parse_operator(p);
            }
            Rule::assert => {
                guard_rule.assert = parse_assert(p);
            }
            Rule::scope => {
                guard_rule.scope = parse_scope(p);
            }
            Rule::should => {
                // should do nothing
            }
            _ => {
                println!("implementing rule: {:?}, level: {:?}", p.as_rule(), p.as_span());
            }
        }
    }

    guard_rule
}

fn parse_operator(parent: Pair<Rule>) -> Vec<Operator> {
    let mut pairs = parent.into_inner();
    let mut pair = pairs.next().unwrap();
    let mut operators: Vec<Operator> = vec![];

    match pair.as_rule() {
        Rule::op_not | Rule::op_not_symbol => {
            operators.push(Operator::Not);
            // get next operator
            pair = pairs.next().unwrap().into_inner().next().unwrap();
        }
        _ => {}
    }

    let ops = match pair.as_rule() {
        Rule::op_lte => { Operator::Lte }
        Rule::op_gte => { Operator::Gte }
        Rule::op_lt => { Operator::Lt }
        Rule::op_gt => { Operator::Gt }
        Rule::op_eq => { Operator::Eq }
        Rule::op_contains => { Operator::Contains }
        Rule::op_endsWith => { Operator::Endswith }
        Rule::op_startsWith => { Operator::StartsWith }
        Rule::op_resideIn => { Operator::ResideIn }
        Rule::op_accessed => { Operator::Accessed }
        Rule::op_dependBy => { Operator::DependBy }
        _ => {
            panic!("implementing ops: {:?}, text: {:?}", pair.as_rule(), pair.as_span())
        }
    };

    operators.push(ops);

    operators
}

fn parse_expr(parent: Pair<Rule>) -> Expr {
    let mut pairs = parent.into_inner();
    let pair = pairs.next().unwrap();

    match pair.as_rule() {
        Rule::fn_call => {
            let mut call_chains: Vec<String> = vec![];

            for p in pair.into_inner() {
                match p.as_rule() {
                    Rule::identifier => {
                        let ident = p.as_span().as_str().to_string();
                        call_chains.push(ident);
                    }
                    _ => {}
                };
            };

            return Expr::PropsCall(call_chains);
        }
        _ => {
            panic!("implementing expr: {:?}, text: {:?}", pair.as_rule(), pair.as_span())
        }
    };
}

fn parse_assert(parent: Pair<Rule>) -> RuleAssert {
    RuleAssert::Empty
}

fn parse_scope(parent: Pair<Rule>) -> RuleScope {
    let mut pairs = parent.into_inner();
    let pair = pairs.next().unwrap();

    match pair.as_rule() {
        Rule::string => {
            let string = unescape(pair.as_str()).expect("incorrect string literal");
            RuleScope::PathDefine(string)
        }
        _ => {
            println!("implementing scope: {:?}, text: {:?}", pair.as_rule(), pair.as_span());
            RuleScope::All
        }
    }
}

fn unescape(string: &str) -> Option<String> {
    let mut result = String::new();
    let mut chars = string.chars();

    loop {
        match chars.next() {
            Some('\\') => match chars.next()? {
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                'r' => result.push('\r'),
                'n' => result.push('\n'),
                't' => result.push('\t'),
                '0' => result.push('\0'),
                '\'' => result.push('\''),
                'x' => {
                    let string: String = chars.clone().take(2).collect();

                    if string.len() != 2 {
                        return None;
                    }

                    for _ in 0..string.len() {
                        chars.next()?;
                    }

                    let value = u8::from_str_radix(&string, 16).ok()?;

                    result.push(char::from(value));
                }
                'u' => {
                    if chars.next()? != '{' {
                        return None;
                    }

                    let string: String = chars.clone().take_while(|c| *c != '}').collect();

                    if string.len() < 2 || 6 < string.len() {
                        return None;
                    }

                    for _ in 0..string.len() + 1 {
                        chars.next()?;
                    }

                    let value = u32::from_str_radix(&string, 16).ok()?;

                    result.push(char::from_u32(value)?);
                }
                _ => return None,
            },
            Some(c) => result.push(c),
            None => return Some(result),
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;
    use crate::parser::ast::{RuleLevel, RuleScope, Expr, Operator};

    #[test]
    fn should_parse_rule_level() {
        let code = "class::name contains \"Controller\";";
        let rules = parse(code);

        assert_eq!(1, rules.len());
        assert_eq!(RuleLevel::Class, rules[0].level);
        assert_eq!(RuleScope::All, rules[0].scope);
    }

    #[test]
    fn should_parse_package_asset() {
        let code = "class(\"..myapp..\")::function.name should contains(\"\");";
        let rules = parse(code);

        assert_eq!(RuleScope::PathDefine(("\"..myapp..\"").to_string()), rules[0].scope);
        let chains = vec!["function".to_string(), "name".to_string()];
        assert_eq!(Expr::PropsCall(chains), rules[0].expr);
    }

    #[test]
    fn should_parse_package_extends() {
        let code = "class(extends \"Connection.class\")::name endsWith \"Connection\";";
        let vec = parse(code);
        assert_eq!(1, vec[0].ops.len());
        assert_eq!(Operator::Endswith, vec[0].ops[0])
    }

    #[test]
    fn should_parse_not_symbol() {
        let code = "class(extends \"Connection.class\")::name should not endsWith \"Connection\";";
        let vec = parse(code);
        assert_eq!(2, vec[0].ops.len());
        assert_eq!(Operator::Not, vec[0].ops[0]);
        assert_eq!(Operator::Endswith, vec[0].ops[1]);
    }

    #[test]
    fn should_parse_package_container_scope() {
        let code = "class(assignable \"EntityManager.class\") resideIn package(\"..persistence.\");";
        parse(code);
    }

    #[test]
    fn should_parse_package_regex() {
        let code = "package(match(\"^/app\")) endsWith \"Connection\";";
        parse(code);
    }

    #[test]
    fn should_parse_class_compare() {
        let code = "class(\"..myapp..\")::function.name should not contains(\"\");
class(\"..myapp..\")::function.name !contains(\"\");

class(\"..myapp..\")::vars.len should <= 20;
class(\"..myapp..\")::function.vars.len should <= 20;
";
        parse(code);
    }

    #[test]
    fn should_parse_simple_usage() {
        let code = "class::name.len should < 20;
function::name.len should < 30;
module::package.len should <= 20;
";
        parse(code);
    }

    #[test]
    fn should_parse_arrow_usage() {
        let code = "class -> name.len should < 20;
function -> name.len should < 30;
module -> package.len should <= 20;
";
        parse(code);
    }

    #[test]
    fn should_parse_layer() {
        let code = "layer(\"onion\")
    ::domainModel(\"\")
    ::domainService(\"\")
    ::applicationService(\"\")
    ::adapter(\"com.phodal.com\", \"zero\");

";
        parse(code);
    }
}