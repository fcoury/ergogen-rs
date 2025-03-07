use std::fmt;

use indexmap::IndexMap;

// Custom error type for the parser
#[derive(Debug)]
pub enum ParserError {
    InvalidCharacter(char),
    SyntaxError(String),
    DivisionByZero,
    UndefinedVariable(String),
    InvalidNumber(String),
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParserError::InvalidCharacter(c) => write!(f, "Invalid character: {}", c),
            ParserError::SyntaxError(msg) => write!(f, "Syntax error: {}", msg),
            ParserError::DivisionByZero => write!(f, "Division by zero"),
            ParserError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            ParserError::InvalidNumber(s) => write!(f, "Invalid number: {}", s),
        }
    }
}

impl std::error::Error for ParserError {}

#[derive(Debug, PartialEq, Clone)]
enum Token {
    Number(f64),
    Identifier(String),
    Plus,
    Minus,
    Multiply,
    Divide,
    LeftParen,
    RightParen,
    Eof,
}

struct Lexer {
    input: Vec<char>,
    position: usize,
    current_char: Option<char>,
}

impl Lexer {
    fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current_char = chars.first().copied();

        Lexer {
            input: chars,
            position: 0,
            current_char,
        }
    }

    fn advance(&mut self) {
        self.position += 1;
        self.current_char = self.input.get(self.position).copied();
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char {
            if !c.is_whitespace() {
                break;
            }
            self.advance();
        }
    }

    fn number(&mut self) -> Result<f64, ParserError> {
        let mut result = String::new();
        let mut has_decimal = false;

        while let Some(c) = self.current_char {
            if c.is_ascii_digit() {
                result.push(c);
                self.advance();
            } else if c == '.' && !has_decimal {
                result.push(c);
                has_decimal = true;
                self.advance();
            } else {
                break;
            }
        }

        result
            .parse::<f64>()
            .map_err(|_| ParserError::InvalidNumber(result))
    }

    fn identifier(&mut self) -> String {
        let mut result = String::new();

        while let Some(c) = self.current_char {
            if c.is_alphanumeric() || c == '_' {
                result.push(c);
                self.advance();
            } else {
                break;
            }
        }

        result
    }

    fn get_next_token(&mut self) -> Result<Token, ParserError> {
        while let Some(c) = self.current_char {
            if c.is_whitespace() {
                self.skip_whitespace();
                continue;
            }

            if c.is_ascii_digit() || c == '.' {
                return Ok(Token::Number(self.number()?));
            }

            if c.is_alphabetic() || c == '_' {
                return Ok(Token::Identifier(self.identifier()));
            }

            match c {
                '+' => {
                    self.advance();
                    return Ok(Token::Plus);
                }
                '-' => {
                    self.advance();
                    return Ok(Token::Minus);
                }
                '*' => {
                    self.advance();
                    return Ok(Token::Multiply);
                }
                '/' => {
                    self.advance();
                    return Ok(Token::Divide);
                }
                '(' => {
                    self.advance();
                    return Ok(Token::LeftParen);
                }
                ')' => {
                    self.advance();
                    return Ok(Token::RightParen);
                }
                _ => return Err(ParserError::InvalidCharacter(c)),
            }
        }

        Ok(Token::Eof)
    }
}

struct Parser {
    lexer: Lexer,
    current_token: Token,
    variables: IndexMap<String, f64>,
}

impl Parser {
    fn new(text: &str, variables: IndexMap<String, f64>) -> Result<Self, ParserError> {
        let mut lexer = Lexer::new(text);
        let current_token = lexer.get_next_token()?;

        Ok(Parser {
            lexer,
            current_token,
            variables,
        })
    }

    fn eat(&mut self, expected_type: &Token) -> Result<(), ParserError> {
        if std::mem::discriminant(&self.current_token) == std::mem::discriminant(expected_type) {
            self.current_token = self.lexer.get_next_token()?;
            Ok(())
        } else {
            Err(ParserError::SyntaxError(format!(
                "Expected {:?}, got {:?}",
                expected_type, self.current_token
            )))
        }
    }

    fn factor(&mut self) -> Result<f64, ParserError> {
        match self.current_token.clone() {
            Token::Number(value) => {
                self.eat(&Token::Number(0.0))?; // The value doesn't matter for eat

                // Handle implicit multiplication: 4.5U
                if let Token::Identifier(var_name) = self.current_token.clone() {
                    self.eat(&Token::Identifier(String::new()))?;
                    if let Some(&var_value) = self.variables.get(&var_name) {
                        return Ok(value * var_value);
                    } else {
                        return Err(ParserError::UndefinedVariable(var_name));
                    }
                }

                Ok(value)
            }
            Token::Identifier(var_name) => {
                self.eat(&Token::Identifier(String::new()))?;
                if let Some(&value) = self.variables.get(&var_name) {
                    Ok(value)
                } else {
                    Err(ParserError::UndefinedVariable(var_name))
                }
            }
            Token::LeftParen => {
                self.eat(&Token::LeftParen)?;
                let result = self.expr()?;
                self.eat(&Token::RightParen)?;
                Ok(result)
            }
            Token::Plus => {
                self.eat(&Token::Plus)?;
                self.factor()
            }
            Token::Minus => {
                self.eat(&Token::Minus)?;
                Ok(-self.factor()?)
            }
            _ => Err(ParserError::SyntaxError(format!(
                "Unexpected token in factor: {:?}",
                self.current_token
            ))),
        }
    }

    fn term(&mut self) -> Result<f64, ParserError> {
        let mut result = self.factor()?;

        while matches!(self.current_token, Token::Multiply | Token::Divide) {
            match self.current_token {
                Token::Multiply => {
                    self.eat(&Token::Multiply)?;
                    result *= self.factor()?;
                }
                Token::Divide => {
                    self.eat(&Token::Divide)?;
                    let divisor = self.factor()?;
                    if divisor == 0.0 {
                        return Err(ParserError::DivisionByZero);
                    }
                    result /= divisor;
                }
                _ => unreachable!(),
            }
        }

        Ok(result)
    }

    fn expr(&mut self) -> Result<f64, ParserError> {
        let mut result = self.term()?;

        while matches!(self.current_token, Token::Plus | Token::Minus) {
            match self.current_token {
                Token::Plus => {
                    self.eat(&Token::Plus)?;
                    result += self.term()?;
                }
                Token::Minus => {
                    self.eat(&Token::Minus)?;
                    result -= self.term()?;
                }
                _ => unreachable!(),
            }
        }

        Ok(result)
    }

    fn parse(&mut self) -> Result<f64, ParserError> {
        let result = self.expr()?;

        // Check if we've consumed all tokens
        if self.current_token != Token::Eof {
            return Err(ParserError::SyntaxError(format!(
                "Unexpected token at end of expression: {:?}",
                self.current_token
            )));
        }

        Ok(result)
    }
}

pub fn evaluate_expression(
    expression: &str,
    variables: &IndexMap<String, f64>,
) -> Result<f64, ParserError> {
    let mut parser = Parser::new(expression, variables.clone())?;
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_expression(expression: &str, variables: IndexMap<String, f64>) -> f64 {
        evaluate_expression(expression, &variables).unwrap()
    }

    #[test]
    fn test_simple_expressions() {
        let variables = IndexMap::new();

        assert_eq!(test_expression("2 + 2", variables.clone()), 4.0);
        assert_eq!(test_expression("30 * 0.5", variables.clone()), 15.0);
        assert_eq!(test_expression("8 / 2", variables.clone()), 4.0);
        assert_eq!(test_expression("19 - 1", variables.clone()), 18.0);
    }

    #[test]
    fn test_variable_expressions() {
        let mut variables = IndexMap::new();
        variables.insert("U".to_string(), 20.0);

        assert_eq!(test_expression("2*U", variables.clone()), 40.0);
        assert_eq!(test_expression("4.5*U", variables.clone()), 90.0);
        assert_eq!(test_expression("U/2", variables.clone()), 10.0);
        assert_eq!(test_expression("U + 15", variables.clone()), 35.0);
        assert_eq!(test_expression("2*U + 5", variables.clone()), 45.0);
        assert_eq!(test_expression("(U + 10) / 2", variables.clone()), 15.0);
    }

    #[test]
    fn test_error_cases() {
        let variables = IndexMap::new();

        assert!(matches!(
            evaluate_expression("2 + * 3", &variables),
            Err(ParserError::SyntaxError(_))
        ));
        assert!(matches!(
            evaluate_expression("3 / 0", &variables),
            Err(ParserError::DivisionByZero)
        ));
        assert!(matches!(
            evaluate_expression("X + 5", &variables),
            Err(ParserError::UndefinedVariable(_))
        ));
        assert!(matches!(
            evaluate_expression("2 + 2)", &variables),
            Err(ParserError::SyntaxError(_))
        ));
    }

    #[test]
    fn test_expression_with_variables() {
        let mut variables = IndexMap::new();
        variables.insert("U".to_string(), 20.0);
        variables.insert("V".to_string(), 10.0);

        assert_eq!(test_expression("U + V", variables.clone()), 30.0);
        assert_eq!(test_expression("U - V", variables.clone()), 10.0);
        assert_eq!(test_expression("U * V", variables.clone()), 200.0);
        assert_eq!(test_expression("U / V", variables.clone()), 2.0);
    }

    #[test]
    fn test_expression_with_custom_units() {
        let mut variables = IndexMap::new();
        variables.insert("U".to_string(), 20.0);
        variables.insert("custom_unit".to_string(), 25.5);

        assert_eq!(test_expression("0.5custom_unit", variables.clone()), 12.75);
        assert_eq!(test_expression("0.5*custom_unit", variables.clone()), 12.75);
        assert_eq!(test_expression("custom_unit+U", variables.clone()), 45.5);
    }

    #[test]
    fn test_expression_with_variables_and_custom_units() {
        let mut variables = IndexMap::new();
        variables.insert("U".to_string(), 20.0);
        variables.insert("custom_unit".to_string(), 25.5);

        assert_eq!(test_expression("custom_unit+U", variables.clone()), 45.5);
        assert_eq!(test_expression("2*U", variables.clone()), 40.0);
        assert_eq!(test_expression("U*custom_unit", variables.clone()), 510.0);
    }

    #[test]
    fn test_expression_with_custom_notation() {
        let mut variables = IndexMap::new();
        variables.insert("U".to_string(), 20.0);
        variables.insert("custom_unit".to_string(), 25.5);

        assert_eq!(test_expression("0.5px", variables.clone()), 20.0);
        assert_eq!(test_expression("2U", variables.clone()), 40.0);
        assert_eq!(test_expression("custom_unit+U", variables.clone()), 45.5);
    }
}

// // Example usage
// fn main() {
//     let mut variables = HashMap::new();
//     variables.insert("U".to_string(), 20.0);
//
//     // Test expressions and handle errors
//     let test_expressions = vec![
//         "2 + 2",
//         "30 * 0.5",
//         "8 / 2",
//         "19 - 1",
//         "2*U",
//         "4.5U",
//         "U/2",
//         "U + 15",
//         "2*U + 5",
//         "(U + 10) / 2",
//         // Error cases
//         "2 + * 3",
//         "U / 0",
//         "X + 5", // Undefined variable
//         "2..5 + 1", // Invalid number
//         "2 + 2)" // Unmatched parenthesis
//     ];
//
//     for expr in test_expressions {
//         match evaluate_expression(expr, variables.clone()) {
//             Ok(result) => println!("{} = {}", expr, result),
//             Err(error) => println!("{} => Error: {}", expr, error),
//         }
//     }
// }
