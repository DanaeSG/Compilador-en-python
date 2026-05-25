// src/analizador_semantico.rs
// ------------------------------------------------------------------------------
//  PUNTOS NEURÁLGICOS DE SEMÁNTICA
//
//  PN-S1  <Programa> start
//         - Crear DirectorioFunciones(programa.nombre).
//
//  PN-S2  Tabla global
//         - Crear y vincular la tabla global al programa.
//
//  PN-S3  Conversión de tipos
//         - Convertir tipo sintáctico -> TipoDato.
//
//  PN-S4  <VARS> globales
//         - Validar doble declaración e insertar en global.
//
//  PN-S5  <FUNC> pre-registro
//         - Preparar el directorio para añadir la función.
//
//  PN-S6  <FUNC> registro
//         - Validar duplicados, guardar retorno y firma.
//
//  PN-S7  Tabla local
//         - Crear y vincular la tabla local a la función.
//
//  PN-S8  <PARAMS>
//         - Convertir tipo, validar duplicados, marcar es_param.
//
//  PN-S9  <VARS> locales
//         - Convertir tipo, validar duplicados, insertar en local.
//
//  PN-S10 Fin de <FUNC>
//         - Cierre conceptual (no-op).
//
//  PN-S11 <FACTOR> id
//         - Resolver local -> global y retornar tipo.
//
//  PN-S12 <EXP> / <TERMINO>
//         - Validar operación aritmética y retornar tipo.
//
//  PN-S13 <EXPRESION> relacional
//         - Validar operación relacional y retornar tipo.
//
//  PN-S14 <ASIGNA>
//         - Validar asignación con cubo semántico.
//
//  PN-S15 <LLAMADA>
//         - Verificar existencia.
//
//  PN-S16 <LLAMADA>
//         - Verificar aridad.
//
//  PN-S17 <LLAMADA>
//         - Verificar tipo de cada argumento con cubo semántico.
//
//  PN-S18 <CONDICION> / <CICLO>
//         - La condición no puede ser nula.
// ------------------------------------------------------------------------------

use crate::ast::*;
use crate::semantica::*;

pub struct AnalizadorSemantico {
    pub directorio: DirectorioFunciones,
    pub cubo:       CuboSemantico,
    pub errores:    Vec<ErrorSemantico>,
}

impl AnalizadorSemantico {
    pub fn new() -> Self {
        Self {
            directorio: DirectorioFunciones::new("__global__"),
            cubo:       CuboSemantico::new(),
            errores:    Vec::new(),
        }
    }

    //  Punto de entrada 
    pub fn analizar(&mut self, prog: &Programa) {
        // PN-S1: Inicializar directorio con el nombre del programa
        self.directorio = DirectorioFunciones::new(&prog.nombre);
        println!("Analizando programa '{}'", prog.nombre);

        // PN-S2: Crear tabla global (se crea en DirectorioFunciones::new)
        // PN-S4: Registrar variables globales
        self.registrar_globales(&prog.vars, &prog.nombre);

        // PN-S5: Preparar directorio para nuevas funciones
        // PN-S6: Registrar funciones antes de analizar cuerpos (llamadas hacia adelante)
        for func in &prog.funcs {
            self.registrar_funcion(func);
        }

        // Analizar cuerpos de funciones
        for func in &prog.funcs {
            self.analizar_cuerpo(&func.cuerpo, &func.nombre);
            // PN-S10: Cierre conceptual de ámbito
            self.cerrar_funcion(&func.nombre);
        }

        // Analizar cuerpo principal
        self.analizar_cuerpo(&prog.cuerpo, &prog.nombre);
    }

    // PN-S4: Registrar variables globales
    fn registrar_globales(&mut self, decls: &[DeclVars], ambito: &str) {
        self.registrar_vars_en_ambito(decls, ambito);
    }

    // PN-S4 / PN-S9: Registrar variables en un ámbito
    fn registrar_vars_en_ambito(&mut self, decls: &[DeclVars], ambito: &str) {
        for decl in decls {
            // PN-S3: Convertir tipo sintáctico -> TipoDato
            let tipo = self.convertir_tipo(&decl.tipo);
            for id in &decl.ids {
                if let Err(e) = self.directorio.declarar_variable(ambito, id, tipo.clone()) {
                    self.errores.push(e);
                }
            }
        }
    }

    // PN-S6 / PN-S7 / PN-S8 / PN-S9: Registrar una función en el directorio
    fn registrar_funcion(&mut self, func: &Funcion) {
        // PN-S3: Convertir tipo de retorno
        let tipo_ret = self.convertir_tipo_func(&func.tipo_retorno);
        // PN-S8: Preparar firma de parámetros (se registran al insertar la función)
        let params = self.registrar_parametros(func);

        // PN-S5: Preparar directorio para nueva función
        // PN-S6: Registrar función y validar duplicados
        // PN-S7: Crear tabla local y vincularla a la función
        match self.directorio.registrar_funcion(&func.nombre, tipo_ret, params) {
            Ok(()) => {
                // PN-S9: Registrar variables locales dentro del ámbito
                self.registrar_locales(&func.vars, &func.nombre);
            }
            Err(e) => self.errores.push(e),
        }
    }

    // PN-S8: Preparar lista de parámetros
    fn registrar_parametros(&self, func: &Funcion) -> Vec<(String, TipoDato)> {
        func.params
            .iter()
            .map(|p| (p.nombre.clone(), self.convertir_tipo(&p.tipo)))
            .collect()
    }

    // PN-S9: Registrar variables locales
    fn registrar_locales(&mut self, decls: &[DeclVars], ambito: &str) {
        self.registrar_vars_en_ambito(decls, ambito);
    }

    // PN-S10: Cerrar el ámbito local de la función (no-op)
    fn cerrar_funcion(&mut self, _ambito: &str) {}

    //  Analizar lista de estatutos 
    fn analizar_cuerpo(&mut self, stmts: &[Estatuto], ambito: &str) {
        for stmt in stmts {
            self.analizar_estatuto(stmt, ambito);
        }
    }

    //  Analizar un estatuto 
    fn analizar_estatuto(&mut self, stmt: &Estatuto, ambito: &str) {
        match stmt {

            // PN-S14: Asignación
            Estatuto::Asigna(id, expr) => {
                self.validar_asignacion(id, expr, ambito);
            }

            // PN-S18: Condición
            Estatuto::Condicion { cond, entonces, sino } => {
                self.verificar_cond_tipo(cond, ambito);
                self.analizar_cuerpo(entonces, ambito);
                if let Some(sino_body) = sino {
                    self.analizar_cuerpo(sino_body, ambito);
                }
            }

            // PN-S18: Ciclo
            Estatuto::Ciclo { cond, cuerpo } => {
                self.verificar_cond_tipo(cond, ambito);
                self.analizar_cuerpo(cuerpo, ambito);
            }

            // PN-S15/PN-S16/PN-S17: Llamada como estatuto
            Estatuto::Llamada(ll) => {
                if let Err(e) = self.inferir_llamada(ll, ambito) {
                    self.errores.push(e);
                }
            }

            // Imprime: verificar tipos de expresiones
            Estatuto::Imprime(alts) => {
                for alt in alts {
                    if let ImprimeAlt::Expr(e) = alt {
                        if let Err(err) = self.inferir_expresion(e, ambito) {
                            self.errores.push(err);
                        }
                    }
                }
            }

            Estatuto::Bloque(stmts) => self.analizar_cuerpo(stmts, ambito),
        }
    }

    fn inferir_expresion(
        &mut self,
        expr: &Expresion,
        ambito: &str,
    ) -> Result<TipoDato, ErrorSemantico> {
        let tipo_izq = self.inferir_exp(&expr.izq, ambito)?;
        match &expr.op {
            None => Ok(tipo_izq),
            Some((op_rel, exp_der)) => {
                let tipo_der = self.inferir_exp(exp_der, ambito)?;
                // PN-S13: Validar operación relacional
                self.validar_operacion_relacional(op_rel, &tipo_izq, &tipo_der)
            }
        }
    }

    fn inferir_exp(&mut self, exp: &Exp, ambito: &str) -> Result<TipoDato, ErrorSemantico> {
        let mut tipo_acc = self.inferir_termino(&exp.termino, ambito)?;
        for (op_arit, term) in &exp.cont {
            let tipo_der = self.inferir_termino(term, ambito)?;
            // PN-S12: Validar operación aritmética
            tipo_acc = self.validar_operacion_aritmetica(op_arit, &tipo_acc, &tipo_der)?;
        }
        Ok(tipo_acc)
    }

    fn inferir_termino(
        &mut self,
        term: &Termino,
        ambito: &str,
    ) -> Result<TipoDato, ErrorSemantico> {
        let mut tipo_acc = self.inferir_factor(&term.factor, ambito)?;
        for (op_mul, fac) in &term.cont {
            let tipo_der = self.inferir_factor(fac, ambito)?;
            // PN-S12: Validar operación aritmética
            tipo_acc = self.validar_operacion_mul_div(op_mul, &tipo_acc, &tipo_der)?;
        }
        Ok(tipo_acc)
    }

    fn inferir_factor(
        &mut self,
        factor: &Factor,
        ambito: &str,
    ) -> Result<TipoDato, ErrorSemantico> {
        match factor {
            Factor::Cte(c) => Ok(match c {
                Constante::Entero(_)   => TipoDato::Entero,
                Constante::Flotante(_) => TipoDato::Flotante,
            }),

            // PN-S11: Id simple
            Factor::Id(id) | Factor::PosId(id) | Factor::NegId(id) => {
                self.resolver_identificador(ambito, id)
            }

            Factor::Paren(expr) => self.inferir_expresion(expr, ambito),

            // PN-S15/PN-S16/PN-S17: Llamada dentro de expresión
            Factor::Llamada(ll) => self.inferir_llamada(ll, ambito),
        }
    }

    fn inferir_llamada(
        &mut self,
        ll: &Llamada,
        ambito: &str,
    ) -> Result<TipoDato, ErrorSemantico> {
        // Built-in: escribe acepta cualquier número/tipo de args
        if ll.nombre == "escribe" {
            for arg in &ll.args {
                let _ = self.inferir_expresion(arg, ambito);
            }
            return Ok(TipoDato::Nula);
        }

        // PN-S15: Verificar existencia de función
        let entrada = self.directorio.buscar_funcion(&ll.nombre)?.clone();

        // PN-S16: Verificar aridad
        if ll.args.len() != entrada.num_params {
            self.errores.push(ErrorSemantico::ArityMismatch {
                funcion:   ll.nombre.clone(),
                esperados: entrada.num_params,
                recibidos: ll.args.len(),
            });
        }

        // PN-S17: Verificar tipo de cada argumento
        for (i, (arg, tipo_param)) in
            ll.args.iter().zip(entrada.tipos_params.iter()).enumerate()
        {
            match self.inferir_expresion(arg, ambito) {
                Ok(tipo_arg) => {
                    // usamos el cubo de asignación para verificar compatibilidad
                    if let Err(_) = self.cubo.consultar(
                        tipo_param, &tipo_arg, &Operador::Asigna
                    ) {
                        self.errores.push(ErrorSemantico::TipoIncompatible {
                            op:  format!("arg {} de '{}'", i + 1, ll.nombre),
                            izq: tipo_param.to_string(),
                            der: tipo_arg.to_string(),
                        });
                    }
                }
                Err(e) => self.errores.push(e),
            }
        }

        Ok(entrada.tipo_retorno.clone())
    }

    // PN-S18: Verificar que condición no sea nula
    fn verificar_cond_tipo(&mut self, cond: &Expresion, ambito: &str) {
        match self.inferir_expresion(cond, ambito) {
            Ok(TipoDato::Nula) => self.errores.push(ErrorSemantico::TipoIncompatible {
                op:  "condición".to_string(),
                izq: "nula".to_string(),
                der: "nula".to_string(),
            }),
            Ok(_) => {}
            Err(e) => self.errores.push(e),
        }
    }

    // PN-S11: Resolver identificador en ámbito visible
    fn resolver_identificador(
        &self,
        ambito: &str,
        id: &str,
    ) -> Result<TipoDato, ErrorSemantico> {
        self.directorio.resolver_variable(ambito, id)
    }

    // PN-S12: Validar operación aritmética (+/-)
    fn validar_operacion_aritmetica(
        &self,
        op_arit: &OpArit,
        tipo_izq: &TipoDato,
        tipo_der: &TipoDato,
    ) -> Result<TipoDato, ErrorSemantico> {
        let op = match op_arit {
            OpArit::Plus => Operador::Suma,
            OpArit::Minus => Operador::Resta,
        };
        self.cubo.consultar(tipo_izq, tipo_der, &op)
            .map_err(|_| ErrorSemantico::TipoIncompatible {
                op:  format!("{:?}", op_arit),
                izq: tipo_izq.to_string(),
                der: tipo_der.to_string(),
            })
    }

    // PN-S12: Validar operación aritmética (*//)
    fn validar_operacion_mul_div(
        &self,
        op_mul: &OpMul,
        tipo_izq: &TipoDato,
        tipo_der: &TipoDato,
    ) -> Result<TipoDato, ErrorSemantico> {
        let op = match op_mul {
            OpMul::Star => Operador::Mul,
            OpMul::Slash => Operador::Div,
        };
        self.cubo.consultar(tipo_izq, tipo_der, &op)
            .map_err(|_| ErrorSemantico::TipoIncompatible {
                op:  format!("{:?}", op_mul),
                izq: tipo_izq.to_string(),
                der: tipo_der.to_string(),
            })
    }

    // PN-S13: Validar operación relacional
    fn validar_operacion_relacional(
        &self,
        op_rel: &OpRel,
        tipo_izq: &TipoDato,
        tipo_der: &TipoDato,
    ) -> Result<TipoDato, ErrorSemantico> {
        let op = match op_rel {
            OpRel::Gt => Operador::Mayor,
            OpRel::Lt => Operador::Menor,
            OpRel::EqEq => Operador::Igual,
            OpRel::Neq => Operador::Diferente,
        };
        self.cubo.consultar(tipo_izq, tipo_der, &op)
            .map_err(|_msg| ErrorSemantico::TipoIncompatible {
                op:  format!("{:?}", op_rel),
                izq: tipo_izq.to_string(),
                der: tipo_der.to_string(),
            })
    }

    // PN-S14: Validar asignación
    fn validar_asignacion(&mut self, id: &str, expr: &Expresion, ambito: &str) {
        let tipo_id = match self.directorio.resolver_variable(ambito, id) {
            Ok(t) => t,
            Err(e) => {
                self.errores.push(e);
                return;
            }
        };

        let tipo_expr = match self.inferir_expresion(expr, ambito) {
            Ok(t) => t,
            Err(e) => {
                self.errores.push(e);
                return;
            }
        };

        if let Err(_) = self.cubo.consultar(&tipo_id, &tipo_expr, &Operador::Asigna) {
            self.errores.push(ErrorSemantico::AsignacionTipoIncompatible {
                var:       id.to_string(),
                var_tipo:  tipo_id.to_string(),
                expr_tipo: tipo_expr.to_string(),
            });
        }
    }

    // PN-S3: Convertir tipo sintáctico -> TipoDato
    fn convertir_tipo(&self, tipo: &Tipo) -> TipoDato {
        TipoDato::from_tipo(tipo)
    }

    // PN-S3: Convertir tipo de retorno sintáctico -> TipoDato
    fn convertir_tipo_func(&self, tipo: &TipoFunc) -> TipoDato {
        TipoDato::from_tipo_func(tipo)
    }

    //  Resultado final 
    pub fn tiene_errores(&self) -> bool { !self.errores.is_empty() }

    pub fn reporte(&self) -> String {
        if self.errores.is_empty() {
            return "Análisis semántico: OK — sin errores.".to_string();
        }
        let mut out = format!("Análisis semántico: {} error(es)\n", self.errores.len());
        for (i, e) in self.errores.iter().enumerate() {
            out.push_str(&format!("  [{}] {}\n", i + 1, e));
        }
        out
    }
}
