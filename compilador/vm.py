"""Máquina virtual y archivo objeto del compilador.

Diseño general:
- La VM interpreta los cuádruplos ya generados por el compilador.
- La memoria se resuelve por dirección virtual, sin arreglos gigantes.
- Cada llamada crea un `ActivationRecord` con locales y temporales propios.
- Las constantes viven en un segmento separado y de solo lectura.
- El mismo modelo sirve tanto para ejecución directa como para archivo `.obj`.
"""

from dataclasses import dataclass, field
import json
from pathlib import Path

from .quadruples import Quadruple, QuadrupleGenerator
from .semantics import DirectorioFunciones, EntradaFuncion, TablaVariables, TipoDato


class VirtualMachineError(Exception):
    """Error de ejecución de la máquina virtual."""

    def __init__(self, kind: str, **data):
        self.kind = kind
        self.data = data
        super().__init__(str(self))

    def __str__(self) -> str:
        data = self.data
        if self.kind == "DireccionInvalida":
            return f"Dirección virtual inválida: {data['direccion']}"
        if self.kind == "VariableNoInicializada":
            return f"Lectura de variable no inicializada en dirección {data['direccion']}"
        if self.kind == "DivisionEntreCero":
            return "División entre cero"
        if self.kind == "StackOverflow":
            return f"Stack overflow de llamadas (límite {data['limite']})"
        if self.kind == "TipoIncompatible":
            return f"Tipo incompatible en dirección {data['direccion']}: valor {data['valor']!r}"
        if self.kind == "ConstanteSoloLectura":
            return f"No se puede escribir en constante {data['direccion']}"
        if self.kind == "CuadruploInvalido":
            return f"Cuádruplo inválido en IP {data['ip']}: {data['op']}"
        if self.kind == "ContextoLlamadaInvalido":
            return "Contexto de llamada inválido"
        return self.kind


def _tipo_desde_direccion(address: int) -> TipoDato | str:
    if 1000 <= address < 2000:
        return TipoDato.ENTERO
    if 2000 <= address < 3000:
        return TipoDato.FLOTANTE
    if 3000 <= address < 4000:
        return TipoDato.ENTERO
    if 4000 <= address < 5000:
        return TipoDato.FLOTANTE
    if 5000 <= address < 6000:
        return TipoDato.ENTERO
    if 6000 <= address < 7000:
        return TipoDato.FLOTANTE
    if 8000 <= address < 9000:
        return TipoDato.ENTERO
    if 9000 <= address < 10000:
        return TipoDato.FLOTANTE
    if address >= 10000:
        return "string"
    raise VirtualMachineError("DireccionInvalida", direccion=address)


def _segmento_desde_direccion(address: int) -> str:
    if 1000 <= address < 3000:
        return "global"
    if 3000 <= address < 5000:
        return "local"
    if 5000 <= address < 7000:
        return "temp"
    if address >= 8000:
        return "const"
    raise VirtualMachineError("DireccionInvalida", direccion=address)


@dataclass
class MemorySegment:
    """Segmento dinámico indexado por dirección virtual.

    Se usa un diccionario porque las direcciones virtuales son dispersas y
    porque solo necesitamos almacenar celdas realmente utilizadas.
    """

    values_by_address: dict[int, object] = field(default_factory=dict)
    read_only: bool = False

    def read(self, address: int) -> object:
        if address not in self.values_by_address:
            raise VirtualMachineError("VariableNoInicializada", direccion=address)
        return self.values_by_address[address]

    def write(self, address: int, value: object) -> None:
        if self.read_only:
            raise VirtualMachineError("ConstanteSoloLectura", direccion=address)
        self.values_by_address[address] = self._coerce(address, value)

    def preload(self, address: int, value: object) -> None:
        self.values_by_address[address] = self._coerce(address, value)

    def _coerce(self, address: int, value: object) -> object:
        tipo = _tipo_desde_direccion(address)
        if tipo == "string":
            if not isinstance(value, str):
                raise VirtualMachineError("TipoIncompatible", direccion=address, valor=value)
            return value
        if tipo == TipoDato.ENTERO:
            if isinstance(value, bool):
                return int(value)
            if isinstance(value, int):
                return value
            if isinstance(value, float) and value.is_integer():
                return int(value)
            raise VirtualMachineError("TipoIncompatible", direccion=address, valor=value)
        if isinstance(value, int):
            return float(value)
        if isinstance(value, float):
            return value
        raise VirtualMachineError("TipoIncompatible", direccion=address, valor=value)


@dataclass
class ActivationRecord:
    """Contexto de ejecución de una función.

    Agrupa los segmentos locales y temporales de una invocación concreta,
    además de la dirección de retorno al terminar la función.
    """

    function_name: str
    return_ip: int | None = None
    locals: MemorySegment = field(default_factory=MemorySegment)
    temps: MemorySegment = field(default_factory=MemorySegment)

    def read(self, address: int) -> object:
        segmento = _segmento_desde_direccion(address)
        if segmento == "local":
            return self.locals.read(address)
        if segmento == "temp":
            return self.temps.read(address)
        raise VirtualMachineError("DireccionInvalida", direccion=address)

    def write(self, address: int, value: object) -> None:
        segmento = _segmento_desde_direccion(address)
        if segmento == "local":
            self.locals.write(address, value)
            return
        if segmento == "temp":
            self.temps.write(address, value)
            return
        raise VirtualMachineError("DireccionInvalida", direccion=address)


@dataclass
class ObjectProgram:
    """Representación serializable de un programa compilado.

    Funciona como frontera estable entre compilación y ejecución: empaqueta
    directorio de funciones, tabla de constantes y cuádruplos.
    """

    directorio: DirectorioFunciones
    constantes: dict[int, object]
    quadruples: list[Quadruple]

    @classmethod
    def from_generator(cls, generator: QuadrupleGenerator) -> "ObjectProgram":
        return cls(
            directorio=generator.directorio,
            constantes=_build_constant_table(generator),
            quadruples=list(generator.quadruples.items),
        )

    def to_dict(self) -> dict[str, object]:
        return {
            "dirfunc": _serialize_directorio(self.directorio),
            "constants": [
                {
                    "address": address,
                    "type": _tipo_serializable(address),
                    "value": value,
                }
                for address, value in sorted(self.constantes.items())
            ],
            "quadruples": [
                {"op": quad.op, "arg1": quad.arg1, "arg2": quad.arg2, "res": quad.res}
                for quad in self.quadruples
            ],
        }

    @classmethod
    def from_dict(cls, data: dict[str, object]) -> "ObjectProgram":
        quads = [
            Quadruple(op=item["op"], arg1=item["arg1"], arg2=item["arg2"], res=item["res"])
            for item in data["quadruples"]
        ]
        constantes = {item["address"]: item["value"] for item in data["constants"]}
        return cls(
            directorio=_deserialize_directorio(data["dirfunc"]),
            constantes=constantes,
            quadruples=quads,
        )

    def save(self, path: str | Path) -> None:
        Path(path).write_text(json.dumps(self.to_dict(), indent=2, ensure_ascii=False), encoding="utf-8")

    @classmethod
    def load(cls, path: str | Path) -> "ObjectProgram":
        data = json.loads(Path(path).read_text(encoding="utf-8"))
        return cls.from_dict(data)


class VirtualMachine:
    """Interpreta cuádruplos usando memoria virtual dinámica.
    """

    def __init__(
        self,
        program: ObjectProgram,
        *,
        max_call_depth: int = 1000,
        input_provider=None,
    ):
        self.program = program
        self.max_call_depth = max_call_depth
        self.input_provider = input_provider or input
        self.globals = MemorySegment()
        self.constants = MemorySegment(read_only=True)
        for address, value in program.constantes.items():
            self.constants.preload(address, value)
        self.frames: list[ActivationRecord] = [ActivationRecord(program.directorio.nombre_prog)]
        self.pending_frames: list[ActivationRecord] = []
        self.ip = 0
        self.output: list[str] = []
        self._current_line: list[str] = []
        self.handlers = {
            "GOTO": self._op_goto,
            "GOTOF": self._op_gotof,
            "GOTOT": self._op_gotot,
            "END": self._op_end,
            "ENDFUNC": self._op_endfunc,
            "PRINT": self._op_print,
            "PRINTS": self._op_prints,
            "READ": self._op_read,
            "=": self._op_assign,
            "+": self._op_add,
            "-": self._op_sub,
            "*": self._op_mul,
            "/": self._op_div,
            ">": self._op_gt,
            "<": self._op_lt,
            "==": self._op_eq,
            "!=": self._op_neq,
            "ERA": self._op_era,
            "PARAM": self._op_param,
            "GOSUB": self._op_gosub,
            "RETURN": self._op_return,
        }

    def run(self) -> list[str]:
        while 0 <= self.ip < len(self.program.quadruples):
            quad = self.program.quadruples[self.ip]
            if quad.op not in {"PRINT", "PRINTS"}:
                self._flush_output_line()
            handler = self.handlers.get(quad.op)
            if handler is None:
                raise VirtualMachineError("CuadruploInvalido", ip=self.ip, op=quad.op)
            handler(quad)
        self._flush_output_line()
        return list(self.output)

    @property
    def current_frame(self) -> ActivationRecord:
        return self.frames[-1]

    def read(self, address: int) -> object:
        segmento = _segmento_desde_direccion(address)
        if segmento == "global":
            return self.globals.read(address)
        if segmento == "const":
            return self.constants.read(address)
        return self.current_frame.read(address)

    def write(self, address: int, value: object) -> None:
        segmento = _segmento_desde_direccion(address)
        if segmento == "global":
            self.globals.write(address, value)
            return
        if segmento == "const":
            self.constants.write(address, value)
            return
        self.current_frame.write(address, value)

    def _flush_output_line(self) -> None:
        if self._current_line:
            self.output.append("".join(self._current_line))
            self._current_line = []

    def _binary_numeric(self, quad: Quadruple) -> tuple[object, object]:
        return self.read(quad.arg1), self.read(quad.arg2)

    def _jump_to_caller(self) -> None:
        if len(self.frames) == 1:
            raise VirtualMachineError("ContextoLlamadaInvalido")
        finished = self.frames.pop()
        if finished.return_ip is None:
            raise VirtualMachineError("ContextoLlamadaInvalido")
        self.ip = finished.return_ip

    def _op_goto(self, quad: Quadruple) -> None:
        self.ip = quad.res

    def _op_gotof(self, quad: Quadruple) -> None:
        condition = self.read(quad.arg1)
        self.ip = quad.res if condition == 0 else self.ip + 1

    def _op_gotot(self, quad: Quadruple) -> None:
        condition = self.read(quad.arg1)
        self.ip = quad.res if condition != 0 else self.ip + 1

    def _op_end(self, quad: Quadruple) -> None:
        self.ip = len(self.program.quadruples)

    def _op_endfunc(self, quad: Quadruple) -> None:
        self._jump_to_caller()

    def _op_print(self, quad: Quadruple) -> None:
        self._current_line.append(str(self.read(quad.arg1)))
        self.ip += 1

    def _op_prints(self, quad: Quadruple) -> None:
        self._current_line.append(str(self.read(quad.arg1)))
        self.ip += 1

    def _op_read(self, quad: Quadruple) -> None:
        raw = self.input_provider()
        tipo = _tipo_desde_direccion(quad.res)
        if tipo == TipoDato.ENTERO:
            value = int(raw)
        elif tipo == TipoDato.FLOTANTE:
            value = float(raw)
        else:
            value = raw
        self.write(quad.res, value)
        self.ip += 1

    def _op_assign(self, quad: Quadruple) -> None:
        self.write(quad.res, self.read(quad.arg1))
        self.ip += 1

    def _op_add(self, quad: Quadruple) -> None:
        left, right = self._binary_numeric(quad)
        self.write(quad.res, left + right)
        self.ip += 1

    def _op_sub(self, quad: Quadruple) -> None:
        left, right = self._binary_numeric(quad)
        self.write(quad.res, left - right)
        self.ip += 1

    def _op_mul(self, quad: Quadruple) -> None:
        left, right = self._binary_numeric(quad)
        self.write(quad.res, left * right)
        self.ip += 1

    def _op_div(self, quad: Quadruple) -> None:
        left, right = self._binary_numeric(quad)
        if right == 0:
            raise VirtualMachineError("DivisionEntreCero")
        if _tipo_desde_direccion(quad.res) == TipoDato.ENTERO:
            self.write(quad.res, int(left / right))
        else:
            self.write(quad.res, float(left) / float(right))
        self.ip += 1

    def _op_gt(self, quad: Quadruple) -> None:
        left, right = self._binary_numeric(quad)
        self.write(quad.res, 1 if left > right else 0)
        self.ip += 1

    def _op_lt(self, quad: Quadruple) -> None:
        left, right = self._binary_numeric(quad)
        self.write(quad.res, 1 if left < right else 0)
        self.ip += 1

    def _op_eq(self, quad: Quadruple) -> None:
        left, right = self._binary_numeric(quad)
        self.write(quad.res, 1 if left == right else 0)
        self.ip += 1

    def _op_neq(self, quad: Quadruple) -> None:
        left, right = self._binary_numeric(quad)
        self.write(quad.res, 1 if left != right else 0)
        self.ip += 1

    def _op_era(self, quad: Quadruple) -> None:
        if len(self.frames) >= self.max_call_depth:
            raise VirtualMachineError("StackOverflow", limite=self.max_call_depth)
        function_name = self.read(quad.arg1)
        self.pending_frames.append(ActivationRecord(function_name=str(function_name)))
        self.ip += 1

    def _op_param(self, quad: Quadruple) -> None:
        if not self.pending_frames:
            raise VirtualMachineError("ContextoLlamadaInvalido")
        self.pending_frames[-1].write(quad.res, self.read(quad.arg1))
        self.ip += 1

    def _op_gosub(self, quad: Quadruple) -> None:
        if not self.pending_frames:
            raise VirtualMachineError("ContextoLlamadaInvalido")
        pending_frame = self.pending_frames.pop()
        pending_frame.return_ip = self.ip + 1
        self.frames.append(pending_frame)
        self.ip = quad.res

    def _op_return(self, quad: Quadruple) -> None:
        function_name = self.current_frame.function_name
        global_entry = self.program.directorio.funciones[self.program.directorio.nombre_prog].tabla_vars.buscar(
            function_name
        )
        self.globals.write(global_entry.dir_virtual, self.read(quad.res))
        self._jump_to_caller()


def _build_constant_table(generator: QuadrupleGenerator) -> dict[int, object]:
    constants: dict[int, object] = {}
    for value, address in generator.memory.const_int.items():
        constants[address] = value
    for value, address in generator.memory.const_float.items():
        constants[address] = value
    for value, address in generator.memory.const_str.items():
        constants[address] = value
    return constants


def _tipo_serializable(address: int) -> str:
    tipo = _tipo_desde_direccion(address)
    return tipo.value if isinstance(tipo, TipoDato) else tipo


def _serialize_directorio(directorio: DirectorioFunciones) -> dict[str, object]:
    return {
        "nombre_prog": directorio.nombre_prog,
        "funciones": {
            nombre: {
                "tipo_retorno": entrada.tipo_retorno.value,
                "num_params": entrada.num_params,
                "dir_inicio": entrada.dir_inicio,
                "tipos_params": [tipo.value for tipo in entrada.tipos_params],
                "nombres_params": list(entrada.nombres_params),
                "tabla_vars": {
                    var_nombre: {
                        "tipo": variable.tipo.value,
                        "es_param": variable.es_param,
                        "dir_virtual": variable.dir_virtual,
                    }
                    for var_nombre, variable in entrada.tabla_vars.variables.items()
                },
            }
            for nombre, entrada in directorio.funciones.items()
        },
    }


def _deserialize_directorio(data: dict[str, object]) -> DirectorioFunciones:
    directorio = DirectorioFunciones(nombre_prog=data["nombre_prog"])
    directorio.funciones = {}
    for nombre, entrada_data in data["funciones"].items():
        tabla = TablaVariables()
        for var_nombre, variable_data in entrada_data["tabla_vars"].items():
            tabla.declarar(
                var_nombre,
                TipoDato(variable_data["tipo"]),
                variable_data["es_param"],
                variable_data["dir_virtual"],
            )
        directorio.funciones[nombre] = EntradaFuncion(
            tipo_retorno=TipoDato(entrada_data["tipo_retorno"]),
            num_params=entrada_data["num_params"],
            dir_inicio=entrada_data["dir_inicio"],
            tipos_params=[TipoDato(tipo) for tipo in entrada_data["tipos_params"]],
            nombres_params=list(entrada_data["nombres_params"]),
            tabla_vars=tabla,
        )
    return directorio
