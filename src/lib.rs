// src/lib.rs

pub mod lexer;
pub mod ast;
pub mod semantica;
pub mod analizador_semantico;
pub mod cuadruplos;
lalrpop_util::lalrpop_mod!(pub gramatica, "/gramatica.rs");

use lexer::Lexer;

// Función principal de parsing.
// Recibe el código fuente como string y devuelve el AST del programa
// o un error formateado en caso de falla sintáctica.
pub fn parse(src: &str) -> Result<ast::Programa, String> {
    let lexer = Lexer::new(src);

    gramatica::ProgramaParser::new()
        .parse(lexer)
        .map_err(|e| format!("{:?}", e)) 
}

