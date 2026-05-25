// src/main.rs

use compilador::analizador_semantico::AnalizadorSemantico;
use compilador::cuadruplos::GeneradorCuadruplos;
use compilador::lexer::Lexer;
use compilador::parse;

const TEST_PROGRAMS: [&str; 7] = [
    "programas_prueba/p1_asignaciones.patito",
    "programas_prueba/p2_relacionales.patito",
    "programas_prueba/p3_parentesis.patito",
    "programas_prueba/p4_imprime.patito",
    "programas_prueba/p5_unarios.patito",
    "programas_prueba/p6_stress_lineal.patito",
    "programas_prueba/p7_nonlineal.patito",
];

fn dump_lexer(src: &str) {
    for item in Lexer::new(src) {
        match item {
            Ok((start, tok, end)) => println!("[{}..{}] {:?}", start, end, tok),
            Err(e) => println!("LEX_ERR {}", e),
        }
    }
}

fn compilar_y_analizar(
    src: &str,
    mostrar_ast: bool,
    mostrar_lex: bool,
    mostrar_semantica: bool,
    mostrar_cuadruplos: bool,
) -> Result<(), String> {
    if mostrar_lex {
        println!("Lexer:");
        dump_lexer(src);
    }

    let ast = parse(src)?;
    if mostrar_ast {
        println!("Parsed OK:\n{:#?}", ast);
    }

    let mut sem = AnalizadorSemantico::new();
    sem.analizar(&ast);
    if mostrar_semantica {
        println!("{}", sem.reporte());
    }

    if sem.tiene_errores() {
        return Err(sem.reporte());
    }

    if mostrar_cuadruplos {
        let mut gen = GeneradorCuadruplos::new(&ast, &sem.directorio, &sem.cubo);
        gen.generar(&ast)?;
        println!("Cuadruplos:\n{}", gen.fila.dump());
    }

    Ok(())
}

fn main() {
    let mut path: Option<String> = None;
    let mut show_ast = false;
    let mut show_lex = false;
    let mut show_sem = false;
    let mut show_quad = false;
    let mut has_output_flag = false;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--ast" => {
                show_ast = true;
                has_output_flag = true;
            }
            "--lex" => {
                show_lex = true;
                has_output_flag = true;
            }
            "--sem" => {
                show_sem = true;
                has_output_flag = true;
            }
            "--quad" => {
                show_quad = true;
                has_output_flag = true;
            }
            _ => {
                if path.is_none() {
                    path = Some(arg);
                }
            }
        }
    }

    if !has_output_flag {
        show_sem = true;
        show_quad = true;
    }

    match path {
        Some(path) => {
            let src = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("No se pudo leer '{}'", path));
            match compilar_y_analizar(&src, show_ast, show_lex, show_sem, show_quad) {
                Ok(()) => {}
                Err(msg) => eprintln!("{}", msg),
            }
        }
        None => {
            for test_path in TEST_PROGRAMS.iter() {
                let src = std::fs::read_to_string(test_path)
                    .unwrap_or_else(|_| panic!("No se pudo leer '{}'", test_path));
                println!("== {} ==", test_path);
                match compilar_y_analizar(&src, show_ast, show_lex, show_sem, show_quad) {
                    Ok(()) => {}
                    Err(msg) => eprintln!("{}", msg),
                }
            }
        }
    }
}
