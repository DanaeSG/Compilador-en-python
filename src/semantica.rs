// src/semantica.rs
// ------------------------------------------------------------------------------
//  TABLA DE CONSIDERACIONES SEMÁNTICAS — "Cubo Semántico"
//
//  El cubo semántico define el tipo resultante de aplicar un operador a dos
//  operandos de tipos dados.  Para el lenguaje Patito existen dos tipos de dato:
//    • Entero   (i)
//    • Flotante (f)
//
//  Indexación: cubo[tipo_izq][tipo_der][op] -> tipo_resultado | Error
// ------------------------------------------------------------------------------

use std::collections::HashMap;
use crate::ast::{Tipo, TipoFunc};

/// Tipo de dato en tiempo de compilación.
/// Nula se usa para funciones void y para "sin tipo todavía".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TipoDato {
    Entero,
    Flotante,
    Nula,
}

impl TipoDato {
    pub fn from_tipo(t: &Tipo) -> Self {
        match t {
            Tipo::Entero   => TipoDato::Entero,
            Tipo::Flotante => TipoDato::Flotante,
        }
    }
    pub fn from_tipo_func(t: &TipoFunc) -> Self {
        match t {
            TipoFunc::Nula    => TipoDato::Nula,
            TipoFunc::Tipo(t) => TipoDato::from_tipo(t),
        }
    }
}

impl std::fmt::Display for TipoDato {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TipoDato::Entero   => write!(f, "entero"),
            TipoDato::Flotante => write!(f, "flotante"),
            TipoDato::Nula     => write!(f, "nula"),
        }
    }
}

/// Operadores del lenguaje.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operador {
    Suma, Resta, Mul, Div,          // aritméticos
    Mayor, Menor, Igual, Diferente, // relacionales
    Asigna,                         // asignación
}

//  Cubo semántico 
//  Representado como HashMap<(TipoDato, TipoDato, Operador), TipoDato>
//  Ausencia de clave -> operación inválida (error semántico).

#[derive(Debug, Clone)]
pub struct CuboSemantico {
    tabla: HashMap<(TipoDato, TipoDato, Operador), TipoDato>,
}

impl CuboSemantico {
    pub fn new() -> Self {
        use Operador::*;
        use TipoDato::*;

        let mut tabla = HashMap::new();

        //  Operadores aritméticos 
        for op in &[Suma, Resta, Mul, Div] {
            tabla.insert((Entero,   Entero,   op.clone()), Entero);
            tabla.insert((Entero,   Flotante, op.clone()), Flotante);
            tabla.insert((Flotante, Entero,   op.clone()), Flotante);
            tabla.insert((Flotante, Flotante, op.clone()), Flotante);
        }

        //  Operadores relacionales 
        for op in &[Mayor, Menor, Igual, Diferente] {
            tabla.insert((Entero,   Entero,   op.clone()), Entero);
            tabla.insert((Entero,   Flotante, op.clone()), Entero);
            tabla.insert((Flotante, Entero,   op.clone()), Entero);
            tabla.insert((Flotante, Flotante, op.clone()), Entero);
        }

        //  Asignación 
        tabla.insert((Entero,   Entero,   Asigna), Entero);
        tabla.insert((Flotante, Entero,   Asigna), Flotante); // promoción ok
        tabla.insert((Flotante, Flotante, Asigna), Flotante);
        // (Entero, Flotante, Asigna) -> ausente = error

        Self { tabla }
    }

    /// Consulta el cubo. Devuelve Ok(tipo_resultado) o Err con mensaje.
    pub fn consultar(
        &self,
        op_izq: &TipoDato,
        op_der: &TipoDato,
        op: &Operador,
    ) -> Result<TipoDato, String> {
        let key = (op_izq.clone(), op_der.clone(), op.clone());
        self.tabla.get(&key).cloned().ok_or_else(|| {
            format!(
                "Operación semántica inválida: {:?} entre '{}' y '{}'",
                op, op_izq, op_der
            )
        })
    }
}

// ------------------------------------------------------------------------------
//  TABLA DE VARIABLES
//
//  Estructura: HashMap<String, EntradaVariable>
//  Clave -> nombre del identificador
//  Valor -> metadatos de la variable
// ------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EntradaVariable {
    pub tipo:   TipoDato,
    pub es_param: bool,    // true si proviene de la lista de parámetros
}

#[derive(Debug, Clone)]
pub struct TablaVariables {
    pub variables: HashMap<String, EntradaVariable>,
}

impl TablaVariables {
    pub fn new() -> Self {
        Self { variables: HashMap::new() }
    }

    /// Declara una variable. Error si ya existía (doble declaración).
    pub fn declarar(
        &mut self,
        nombre: &str,
        tipo: TipoDato,
        es_param: bool,
    ) -> Result<(), ErrorSemantico> {
        if self.variables.contains_key(nombre) {
            return Err(ErrorSemantico::VariableDoblementeDeclada(nombre.to_string()));
        }
        self.variables.insert(
            nombre.to_string(),
            EntradaVariable { tipo, es_param },
        );
        Ok(())
    }

    /// Busca una variable. Error si no existe (variable no declarada).
    pub fn buscar(&self, nombre: &str) -> Result<&EntradaVariable, ErrorSemantico> {
        self.variables.get(nombre)
            .ok_or_else(|| ErrorSemantico::VariableNoDeclarada(nombre.to_string()))
    }
}

// ------------------------------------------------------------------------------
//  DIRECTORIO DE FUNCIONES
//
//  Estructura: HashMap<String, EntradaFuncion>
//  Clave -> nombre de la función
//  Valor -> tipo de retorno + tabla de variables local + num de params
// ------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EntradaFuncion {
    pub tipo_retorno:   TipoDato,
    pub num_params:     usize,
    pub tipos_params:   Vec<TipoDato>,   // orden de los parámetros
    pub tabla_vars:     TablaVariables,
}

#[derive(Debug)]
#[derive(Clone)]
pub struct DirectorioFunciones {
    pub funciones:  HashMap<String, EntradaFuncion>,
    pub nombre_prog: String,             // clave del ámbito global
}

impl DirectorioFunciones {
    pub fn new(nombre_prog: &str) -> Self {
        let mut dir = Self {
            funciones: HashMap::new(),
            nombre_prog: nombre_prog.to_string(),
        };
        // Registrar el ámbito global con clave = nombre del programa
        dir.funciones.insert(
            nombre_prog.to_string(),
            EntradaFuncion {
                tipo_retorno: TipoDato::Nula,
                num_params: 0,
                tipos_params: vec![],
                tabla_vars: TablaVariables::new(),
            },
        );
        dir
    }

    /// Registra una nueva función. Error si ya existe.
    pub fn registrar_funcion(
        &mut self,
        nombre: &str,
        tipo_retorno: TipoDato,
        params: Vec<(String, TipoDato)>,
    ) -> Result<(), ErrorSemantico> {
        if self.funciones.contains_key(nombre) {
            return Err(ErrorSemantico::FuncionDoblementeDeclada(nombre.to_string()));
        }
        let num_params   = params.len();
        let tipos_params = params.iter().map(|(_, t)| t.clone()).collect();
        let mut tabla    = TablaVariables::new();
        // Los parámetros van a la tabla de variables local marcados como param
        for (id, tipo) in params {
            tabla.declarar(&id, tipo, true)?;
        }
        self.funciones.insert(
            nombre.to_string(),
            EntradaFuncion { tipo_retorno, num_params, tipos_params, tabla_vars: tabla },
        );
        Ok(())
    }

    /// Declara una variable en el ámbito de una función (o global).
    pub fn declarar_variable(
        &mut self,
        ambito: &str,
        nombre: &str,
        tipo: TipoDato,
    ) -> Result<(), ErrorSemantico> {
        let entrada = self.funciones.get_mut(ambito)
            .ok_or_else(|| ErrorSemantico::AmbitoNoEncontrado(ambito.to_string()))?;
        entrada.tabla_vars.declarar(nombre, tipo, false)
    }

    /// Busca una función. Error si no existe.
    pub fn buscar_funcion(&self, nombre: &str) -> Result<&EntradaFuncion, ErrorSemantico> {
        self.funciones.get(nombre)
            .ok_or_else(|| ErrorSemantico::FuncionNoDeclarada(nombre.to_string()))
    }

    /// Resuelve el tipo de una variable buscando primero en el ámbito local
    /// y luego en el global.
    pub fn resolver_variable(
        &self,
        ambito_local: &str,
        nombre: &str,
    ) -> Result<TipoDato, ErrorSemantico> {
        // 1. buscar en ámbito local
        if let Ok(ef) = self.buscar_funcion(ambito_local) {
            if let Ok(ev) = ef.tabla_vars.buscar(nombre) {
                return Ok(ev.tipo.clone());
            }
        }
        // 2. buscar en ámbito global
        if ambito_local != self.nombre_prog {
            let global = self.funciones.get(&self.nombre_prog).unwrap();
            if let Ok(ev) = global.tabla_vars.buscar(nombre) {
                return Ok(ev.tipo.clone());
            }
        }
        Err(ErrorSemantico::VariableNoDeclarada(nombre.to_string()))
    }
}

// ------------------------------------------------------------------------------
//  ERRORES SEMÁNTICOS
// ------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ErrorSemantico {
    VariableDoblementeDeclada(String),
    VariableNoDeclarada(String),
    FuncionDoblementeDeclada(String),
    FuncionNoDeclarada(String),
    AmbitoNoEncontrado(String),
    TipoIncompatible { op: String, izq: String, der: String },
    ArityMismatch { funcion: String, esperados: usize, recibidos: usize },
    AsignacionTipoIncompatible { var: String, var_tipo: String, expr_tipo: String },
}

impl std::fmt::Display for ErrorSemantico {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VariableDoblementeDeclada(n) =>
                write!(f, "Variable doblemente declarada: '{}'", n),
            Self::VariableNoDeclarada(n) =>
                write!(f, "Variable no declarada: '{}'", n),
            Self::FuncionDoblementeDeclada(n) =>
                write!(f, "Función doblemente declarada: '{}'", n),
            Self::FuncionNoDeclarada(n) =>
                write!(f, "Función no declarada: '{}'", n),
            Self::AmbitoNoEncontrado(n) =>
                write!(f, "Ámbito no encontrado: '{}'", n),
            Self::TipoIncompatible { op, izq, der } =>
                write!(f, "Tipo incompatible: {} {} {}", izq, op, der),
            Self::ArityMismatch { funcion, esperados, recibidos } =>
                write!(f, "Función '{}': se esperaban {} args, se recibieron {}", funcion, esperados, recibidos),
            Self::AsignacionTipoIncompatible { var, var_tipo, expr_tipo } =>
                write!(f, "No se puede asignar '{}' a variable '{}' de tipo '{}'", expr_tipo, var, var_tipo),
        }
    }
}
