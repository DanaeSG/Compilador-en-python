// src/cuadruplos.rs
// ------------------------------------------------------------------------------
//  PUNTOS NEURÁLGICOS DE CUÁDRUPLOS
// PN-C1   <Programa> <VARS> globales
//         Inicializar memoria virtual y preasignar globales.

// PN-C2   <FUNC> <PARAMS> y <VARS> locales
//         Preasignar parámetros y variables locales.

// PN-C3   <FACTOR> constante
//         Push constante en PilaO y PTypes.

// PN-C4   <FACTOR> id
//         Push identificador en PilaO y PTypes.

// PN-C5   <FACTOR> -id
//         Generar negación unaria como 0 - id.

// PN-C6   <EXP> operador + o -
//         Push + o - en POper.

// PN-C7   <TERMINO> operador * o /
//         Push * o / en POper.

// PN-C8   <EXP>
//         Reducir + o -.

// PN-C9   <TERMINO>
//         Reducir * o /.

// PN-C10  <EXPRESION> relacional
//         Push operador relacional.

// PN-C11  <EXPRESION> relacional
//         Reducir comparación.

// PN-C12  <ASIGNA>
//         Generar cuadruplo de asignación.

// PN-C13  <IMPRIME> expresión
//         Generar PRINT.

// PN-C14  <IMPRIME> letrero
//         Generar PRINTS.
// ------------------------------------------------------------------------------

use std::collections::{HashMap, VecDeque};

use crate::ast::*;
use crate::semantica::{CuboSemantico, DirectorioFunciones, Operador, TipoDato};

#[derive(Debug, Clone)]
pub enum OperadorCuadruplo {
    Suma,
    Resta,
    Mul,
    Div,
    Mayor,
    Menor,
    Igual,
    Diferente,
    Asigna,
    Print,
    PrintStr,
}

impl std::fmt::Display for OperadorCuadruplo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            OperadorCuadruplo::Suma => "+",
            OperadorCuadruplo::Resta => "-",
            OperadorCuadruplo::Mul => "*",
            OperadorCuadruplo::Div => "/",
            OperadorCuadruplo::Mayor => ">",
            OperadorCuadruplo::Menor => "<",
            OperadorCuadruplo::Igual => "==",
            OperadorCuadruplo::Diferente => "!=",
            OperadorCuadruplo::Asigna => "=",
            OperadorCuadruplo::Print => "PRINT",
            OperadorCuadruplo::PrintStr => "PRINTS",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
pub struct Cuadruplo {
    pub op: OperadorCuadruplo,
    pub arg1: Option<i32>,
    pub arg2: Option<i32>,
    pub res: Option<i32>,
}

impl std::fmt::Display for Cuadruplo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let a1 = self.arg1.map_or("_".to_string(), |v| v.to_string());
        let a2 = self.arg2.map_or("_".to_string(), |v| v.to_string());
        let r = self.res.map_or("_".to_string(), |v| v.to_string());
        write!(f, "({}, {}, {}, {})", self.op, a1, a2, r)
    }
}

#[derive(Debug, Clone)]
pub struct FilaCuadruplos {
    pub items: VecDeque<Cuadruplo>,
}

impl FilaCuadruplos {
    pub fn new() -> Self {
        Self { items: VecDeque::new() }
    }

    pub fn push(&mut self, q: Cuadruplo) {
        self.items.push_back(q);
    }

    pub fn dump(&self) -> String {
        let mut out = String::new();
        for (i, q) in self.items.iter().enumerate() {
            out.push_str(&format!("[{}] {}\n", i, q));
        }
        out
    }
}

// Tabla de direcciones de memoria
// (rangos definidos para cada tipo y segmento)
// +----------------------+-------------------+
// | Segmento             | Rango base        |
// +----------------------+-------------------+
// | Global entero        | 1000..            |
// | Global flotante      | 2000..            |
// | Local entero         | 3000..            |
// | Local flotante       | 4000..            |
// | Temporal entero      | 5000..            |
// | Temporal flotante    | 6000..            |
// | Constante entero     | 7000..            |
// | Constante flotante   | 8000..            |
// | Constante string     | 9000..            |
// +----------------------+-------------------+
#[derive(Debug, Clone)]
struct Memoria {
    global: HashMap<String, i32>,
    local: HashMap<String, HashMap<String, i32>>,
    const_int: HashMap<i64, i32>,
    const_float: HashMap<u64, i32>,
    const_str: HashMap<String, i32>,
    next_global_int: i32,
    next_global_float: i32,
    next_local_int: i32,
    next_local_float: i32,
    next_temp_int: i32,
    next_temp_float: i32,
    next_const_int: i32,
    next_const_float: i32,
    next_const_str: i32,
}

impl Memoria {
    fn new() -> Self {
        Self {
            global: HashMap::new(),
            local: HashMap::new(),
            const_int: HashMap::new(),
            const_float: HashMap::new(),
            const_str: HashMap::new(),
            next_global_int: 1000,
            next_global_float: 2000,
            next_local_int: 3000,
            next_local_float: 4000,
            next_temp_int: 5000,
            next_temp_float: 6000,
            next_const_int: 7000,
            next_const_float: 8000,
            next_const_str: 9000,
        }
    }

    fn alloc_global(&mut self, nombre: &str, tipo: &TipoDato) -> i32 {
        if let Some(dir) = self.global.get(nombre) {
            return *dir;
        }
        let dir = match tipo {
            TipoDato::Entero => {
                let d = self.next_global_int;
                self.next_global_int += 1;
                d
            }
            TipoDato::Flotante => {
                let d = self.next_global_float;
                self.next_global_float += 1;
                d
            }
            TipoDato::Nula => {
                let d = self.next_global_int;
                self.next_global_int += 1;
                d
            }
        };
        self.global.insert(nombre.to_string(), dir);
        dir
    }

    fn alloc_local(&mut self, ambito: &str, nombre: &str, tipo: &TipoDato) -> i32 {
        let mapa = self.local.entry(ambito.to_string()).or_insert_with(HashMap::new);
        if let Some(dir) = mapa.get(nombre) {
            return *dir;
        }
        let dir = match tipo {
            TipoDato::Entero => {
                let d = self.next_local_int;
                self.next_local_int += 1;
                d
            }
            TipoDato::Flotante => {
                let d = self.next_local_float;
                self.next_local_float += 1;
                d
            }
            TipoDato::Nula => {
                let d = self.next_local_int;
                self.next_local_int += 1;
                d
            }
        };
        mapa.insert(nombre.to_string(), dir);
        dir
    }

    fn get_var(&self, ambito: &str, nombre: &str) -> Option<i32> {
        if let Some(mapa) = self.local.get(ambito) {
            if let Some(dir) = mapa.get(nombre) {
                return Some(*dir);
            }
        }
        self.global.get(nombre).copied()
    }

    fn alloc_temp(&mut self, tipo: &TipoDato) -> i32 {
        match tipo {
            TipoDato::Entero => {
                let d = self.next_temp_int;
                self.next_temp_int += 1;
                d
            }
            TipoDato::Flotante => {
                let d = self.next_temp_float;
                self.next_temp_float += 1;
                d
            }
            TipoDato::Nula => {
                let d = self.next_temp_int;
                self.next_temp_int += 1;
                d
            }
        }
    }

    fn alloc_const_int(&mut self, v: i64) -> i32 {
        if let Some(dir) = self.const_int.get(&v) {
            return *dir;
        }
        let d = self.next_const_int;
        self.next_const_int += 1;
        self.const_int.insert(v, d);
        d
    }

    fn alloc_const_float(&mut self, v: f64) -> i32 {
        let key = v.to_bits();
        if let Some(dir) = self.const_float.get(&key) {
            return *dir;
        }
        let d = self.next_const_float;
        self.next_const_float += 1;
        self.const_float.insert(key, d);
        d
    }

    fn alloc_const_str(&mut self, v: &str) -> i32 {
        if let Some(dir) = self.const_str.get(v) {
            return *dir;
        }
        let d = self.next_const_str;
        self.next_const_str += 1;
        self.const_str.insert(v.to_string(), d);
        d
    }
}

#[derive(Debug)]
pub struct GeneradorCuadruplos {
    directorio: DirectorioFunciones,
    cubo: CuboSemantico,
    pila_operadores: Vec<Operador>,
    pila_operandos: Vec<i32>,
    pila_tipos: Vec<TipoDato>,
    pub fila: FilaCuadruplos,
    memoria: Memoria,
    nombre_global: String,
}

impl GeneradorCuadruplos {
    pub fn new(prog: &Programa, directorio: &DirectorioFunciones, cubo: &CuboSemantico) -> Self {
        let mut gen = Self {
            directorio: directorio.clone(),
            cubo: cubo.clone(),
            pila_operadores: Vec::new(),
            pila_operandos: Vec::new(),
            pila_tipos: Vec::new(),
            fila: FilaCuadruplos::new(),
            memoria: Memoria::new(),
            nombre_global: prog.nombre.clone(),
        };

        // PN-C1: Inicializar memoria virtual y preasignar globales
        gen.preasignar_variables(prog);
        gen
    }

    fn preasignar_variables(&mut self, prog: &Programa) {
        // PN-C1: Globales
        for decl in &prog.vars {
            let tipo = TipoDato::from_tipo(&decl.tipo);
            for id in &decl.ids {
                self.memoria.alloc_global(id, &tipo);
            }
        }

        // PN-C2: Locales y parametros
        for func in &prog.funcs {
            self.preasignar_funcion(func);
        }
    }

    // PN-C2: Preasignar direcciones para locales y parametros
    fn preasignar_funcion(&mut self, func: &Funcion) {
        let ambito = &func.nombre;
        for p in &func.params {
            let tipo = TipoDato::from_tipo(&p.tipo);
            self.memoria.alloc_local(ambito, &p.nombre, &tipo);
        }
        for decl in &func.vars {
            let tipo = TipoDato::from_tipo(&decl.tipo);
            for id in &decl.ids {
                self.memoria.alloc_local(ambito, id, &tipo);
            }
        }
    }

    pub fn generar(&mut self, prog: &Programa) -> Result<(), String> {
        for func in &prog.funcs {
            self.generar_cuerpo(&func.cuerpo, &func.nombre)?;
        }
        self.generar_cuerpo(&prog.cuerpo, &prog.nombre)?;
        Ok(())
    }

    fn generar_cuerpo(&mut self, stmts: &[Estatuto], ambito: &str) -> Result<(), String> {
        for stmt in stmts {
            self.generar_estatuto(stmt, ambito)?;
        }
        Ok(())
    }

    fn generar_estatuto(&mut self, stmt: &Estatuto, ambito: &str) -> Result<(), String> {
        match stmt {
            Estatuto::Asigna(id, expr) => self.generar_asignacion(id, expr, ambito),
            Estatuto::Imprime(alts) => self.generar_imprime(alts, ambito),
            Estatuto::Bloque(stmts) => self.generar_cuerpo(stmts, ambito),
            Estatuto::Llamada(_) => Err("Llamadas no soportadas en cuadruplos lineales".to_string()),
            Estatuto::Condicion { .. } => Err("Condiciones no soportadas en cuadruplos lineales".to_string()),
            Estatuto::Ciclo { .. } => Err("Ciclos no soportados en cuadruplos lineales".to_string()),
        }
    }

    // PN-C12: Generar cuadruplo de asignacion
    fn generar_asignacion(&mut self, id: &str, expr: &Expresion, ambito: &str) -> Result<(), String> {
        self.procesar_expresion(expr, ambito)?;
        self.generar_asignacion_cuadruplo(id, ambito)?;
        Ok(())
    }

    // PN-C12: Cuadruplo de asignacion (post-expresion)
    fn generar_asignacion_cuadruplo(&mut self, id: &str, ambito: &str) -> Result<(), String> {
        let (dir_expr, tipo_expr) = self.pop_operando()?;
        let tipo_id = self.directorio
            .resolver_variable(ambito, id)
            .map_err(|e| e.to_string())?;
        self.cubo
            .consultar(&tipo_id, &tipo_expr, &Operador::Asigna)
            .map_err(|e| e.to_string())?;
        let dir_id = self
            .memoria
            .get_var(ambito, id)
            .or_else(|| self.memoria.get_var(&self.nombre_global, id))
            .ok_or_else(|| format!("Variable no encontrada: {}", id))?;

        self.fila.push(Cuadruplo {
            op: OperadorCuadruplo::Asigna,
            arg1: Some(dir_expr),
            arg2: None,
            res: Some(dir_id),
        });
        Ok(())
    }

    fn generar_imprime(&mut self, alts: &[ImprimeAlt], ambito: &str) -> Result<(), String> {
        for alt in alts {
            match alt {
                ImprimeAlt::Expr(expr) => {
                    // PN-C13: PRINT expresion
                    self.procesar_expresion(expr, ambito)?;
                    self.generar_print()?;
                }
                ImprimeAlt::Letrero(s) => {
                    // PN-C14: PRINTS string
                    self.generar_prints(s);
                }
            }
        }
        Ok(())
    }

    // PN-C13: Generar cuadruplo PRINT
    fn generar_print(&mut self) -> Result<(), String> {
        let (dir_expr, _tipo) = self.pop_operando()?;
        self.fila.push(Cuadruplo {
            op: OperadorCuadruplo::Print,
            arg1: Some(dir_expr),
            arg2: None,
            res: None,
        });
        Ok(())
    }

    // PN-C14: Generar cuadruplo PRINTS
    fn generar_prints(&mut self, s: &str) {
        let dir = self.memoria.alloc_const_str(s);
        self.fila.push(Cuadruplo {
            op: OperadorCuadruplo::PrintStr,
            arg1: Some(dir),
            arg2: None,
            res: None,
        });
    }

    fn procesar_expresion(&mut self, expr: &Expresion, ambito: &str) -> Result<(), String> {
        self.procesar_exp(&expr.izq, ambito)?;
        if let Some((op_rel, exp_der)) = &expr.op {
            let op = match op_rel {
                OpRel::Gt => Operador::Mayor,
                OpRel::Lt => Operador::Menor,
                OpRel::EqEq => Operador::Igual,
                OpRel::Neq => Operador::Diferente,
            };
            // PN-C10: Push operador relacional
            self.push_operador(op);
            self.procesar_exp(exp_der, ambito)?;
            // PN-C11: Reduccion de expresion
            self.reducir_expresion()?;
        }
        Ok(())
    }

    fn procesar_exp(&mut self, exp: &Exp, ambito: &str) -> Result<(), String> {
        // La precedencia se respeta por el orden del AST (<TERMINO> antes de <EXP>).
        self.procesar_termino(&exp.termino, ambito)?;
        for (op_arit, term) in &exp.cont {
            let op = match op_arit {
                OpArit::Plus => Operador::Suma,
                OpArit::Minus => Operador::Resta,
            };
            // PN-C6: Push operador aritmetico
            self.push_operador(op);
            self.procesar_termino(term, ambito)?;
            // PN-C8: Reduccion de expresion
            self.reducir_expresion()?;
        }
        Ok(())
    }

    fn procesar_termino(&mut self, term: &Termino, ambito: &str) -> Result<(), String> {
        self.procesar_factor(&term.factor, ambito)?;
        for (op_mul, fac) in &term.cont {
            let op = match op_mul {
                OpMul::Star => Operador::Mul,
                OpMul::Slash => Operador::Div,
            };
            // PN-C7: Push operador aritmetico
            self.push_operador(op);
            self.procesar_factor(fac, ambito)?;
            // PN-C9: Reduccion de expresion
            self.reducir_expresion()?;
        }
        Ok(())
    }

    fn procesar_factor(&mut self, factor: &Factor, ambito: &str) -> Result<(), String> {
        match factor {
            Factor::Cte(Constante::Entero(v)) => {
                // PN-C3: Push constante
                self.push_constante_entero(*v);
                Ok(())
            }
            Factor::Cte(Constante::Flotante(v)) => {
                // PN-C3: Push constante
                self.push_constante_flotante(*v);
                Ok(())
            }
            Factor::Id(id) | Factor::PosId(id) => {
                // PN-C4: Push variable
                self.push_variable(id, ambito)
            }
            Factor::NegId(id) => {
                // PN-C5: Negacion unaria como 0 - id
                self.generar_negacion_unaria(id, ambito)
            }
            Factor::Paren(expr) => self.procesar_expresion(expr, ambito),
            Factor::Llamada(_) => Err("Llamadas en expresiones no soportadas".to_string()),
        }
    }

    // PN-C3: Push constante entera
    fn push_constante_entero(&mut self, v: i64) {
        let dir = self.memoria.alloc_const_int(v);
        self.pila_operandos.push(dir);
        self.pila_tipos.push(TipoDato::Entero);
    }

    // PN-C3: Push constante flotante
    fn push_constante_flotante(&mut self, v: f64) {
        let dir = self.memoria.alloc_const_float(v);
        self.pila_operandos.push(dir);
        self.pila_tipos.push(TipoDato::Flotante);
    }

    // PN-C4: Push variable
    fn push_variable(&mut self, id: &str, ambito: &str) -> Result<(), String> {
        let tipo = self.directorio.resolver_variable(ambito, id)
            .map_err(|e| e.to_string())?;
        let dir = self
            .memoria
            .get_var(ambito, id)
            .or_else(|| self.memoria.get_var(&self.nombre_global, id))
            .ok_or_else(|| format!("Variable no encontrada: {}", id))?;
        self.pila_operandos.push(dir);
        self.pila_tipos.push(tipo);
        Ok(())
    }

    // PN-C6 / PN-C7 / PN-C10: Push operador
    fn push_operador(&mut self, op: Operador) {
        self.pila_operadores.push(op);
    }

    // PN-C5: Generar negacion unaria como 0 - id
    fn generar_negacion_unaria(&mut self, id: &str, ambito: &str) -> Result<(), String> {
        let tipo = self.directorio.resolver_variable(ambito, id)
            .map_err(|e| e.to_string())?;
        let dir_id = self
            .memoria
            .get_var(ambito, id)
            .or_else(|| self.memoria.get_var(&self.nombre_global, id))
            .ok_or_else(|| format!("Variable no encontrada: {}", id))?;

        let dir_cero = match tipo {
            TipoDato::Entero => self.memoria.alloc_const_int(0),
            TipoDato::Flotante => self.memoria.alloc_const_float(0.0),
            TipoDato::Nula => self.memoria.alloc_const_int(0),
        };

        self.pila_operandos.push(dir_cero);
        self.pila_tipos.push(tipo.clone());
        self.pila_operandos.push(dir_id);
        self.pila_tipos.push(tipo.clone());
        self.push_operador(Operador::Resta);
        self.reducir_expresion()?;
        Ok(())
    }

    // PN-C8 / PN-C9 / PN-C11: Reduccion de expresion
    fn reducir_expresion(&mut self) -> Result<(), String> {
        self.reducir_top()
    }

    fn reducir_top(&mut self) -> Result<(), String> {
        let op = self.pila_operadores.pop().ok_or_else(|| "Pila de operadores vacia".to_string())?;
        let (dir_der, tipo_der) = self.pop_operando()?;
        let (dir_izq, tipo_izq) = self.pop_operando()?;

        let tipo_res = self
            .cubo
            .consultar(&tipo_izq, &tipo_der, &op)
            .map_err(|e| e.to_string())?;
        let dir_temp = self.memoria.alloc_temp(&tipo_res);

        let op_cuad = match op {
            Operador::Suma => OperadorCuadruplo::Suma,
            Operador::Resta => OperadorCuadruplo::Resta,
            Operador::Mul => OperadorCuadruplo::Mul,
            Operador::Div => OperadorCuadruplo::Div,
            Operador::Mayor => OperadorCuadruplo::Mayor,
            Operador::Menor => OperadorCuadruplo::Menor,
            Operador::Igual => OperadorCuadruplo::Igual,
            Operador::Diferente => OperadorCuadruplo::Diferente,
            Operador::Asigna => OperadorCuadruplo::Asigna,
        };

        self.fila.push(Cuadruplo {
            op: op_cuad,
            arg1: Some(dir_izq),
            arg2: Some(dir_der),
            res: Some(dir_temp),
        });

        self.pila_operandos.push(dir_temp);
        self.pila_tipos.push(tipo_res);
        Ok(())
    }

    fn pop_operando(&mut self) -> Result<(i32, TipoDato), String> {
        let dir = self.pila_operandos.pop().ok_or_else(|| "Pila de operandos vacia".to_string())?;
        let tipo = self.pila_tipos.pop().ok_or_else(|| "Pila de tipos vacia".to_string())?;
        Ok((dir, tipo))
    }
}
