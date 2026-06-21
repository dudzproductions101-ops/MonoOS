// CalcEngine.qml – Calculator business logic singleton
pragma Singleton
import QtQuick 2.15

QtObject {
    id: root

    property string display: "0"
    property string expression: ""

    property real _acc: 0
    property string _op: ""
    property bool _newEntry: true

    function input(key) {
        switch(key) {
            case "clear":   _clear();   break
            case "negate":  _negate();  break
            case "percent": _percent(); break
            case "+": case "-": case "*": case "/": _setOp(key); break
            case "=": _equals(); break
            case ".": _decimal(); break
            default:  _digit(key); break
        }
    }

    function _clear() {
        display = "0"; expression = ""; _acc = 0; _op = ""; _newEntry = true
    }

    function _digit(d) {
        if (_newEntry) { display = d; _newEntry = false }
        else display = display === "0" ? d : display + d
    }

    function _decimal() {
        if (_newEntry) { display = "0."; _newEntry = false }
        else if (!display.includes(".")) display += "."
    }

    function _negate() { if (parseFloat(display) !== 0) display = String(-parseFloat(display)) }
    function _percent() { display = String(parseFloat(display) / 100) }

    function _setOp(op) {
        _acc = parseFloat(display)
        _op  = op
        expression = display + " " + op
        _newEntry = true
    }

    function _equals() {
        if (_op === "") return
        var b = parseFloat(display)
        var result = 0
        if (_op === "+") result = _acc + b
        else if (_op === "-") result = _acc - b
        else if (_op === "*") result = _acc * b
        else if (_op === "/" && b !== 0) result = _acc / b
        else { display = "Error"; expression = ""; _op = ""; return }
        expression = expression + " " + display + " ="
        display = String(parseFloat(result.toPrecision(12)))
        _op = ""; _newEntry = true
    }
}
