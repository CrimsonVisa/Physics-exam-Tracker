import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from 'react';
import { isEditableTarget } from '../lib/appShortcuts';
import { CalculatorDisplay } from './calculator/CalculatorDisplay';
import { CalculatorHistory } from './calculator/CalculatorHistory';
import {
  CalculatorKeypad,
  type CalculatorKeyAction,
  type MemoryOperation,
} from './calculator/CalculatorKeypad';
import { applyRepeatedEquals, evaluate } from './calculator/engine';
import { calculationErrorMessage } from './calculator/errors';
import { formatNumber } from './calculator/format';
import { createHistoryEntry } from './calculator/history';
import { useCalculatorSession } from './calculator/session';
import type { BinaryOperator, HistoryEntry, LastBinaryOperation } from './calculator/types';

type DisplayResult = {
  value: number;
  text: string;
  approximate: boolean;
};

function operatorLabel(operator: BinaryOperator): string {
  return operator === 'mod' ? 'mod' : operator;
}

const MEMORY_VALUE_ERROR = 'Memory can only store finite numbers.';

function fallbackCopyText(text: string): void {
  const textarea = document.createElement('textarea');
  textarea.value = text;
  textarea.setAttribute('readonly', '');
  textarea.style.position = 'fixed';
  textarea.style.opacity = '0';
  document.body.appendChild(textarea);
  try {
    textarea.select();
    const copied =
      typeof document.execCommand === 'function' ? document.execCommand('copy') : false;
    if (!copied) {
      throw new Error('Clipboard copy failed');
    }
  } finally {
    document.body.removeChild(textarea);
  }
}

export function CalculatorScreen() {
  const [expression, setExpression] = useState('');
  const [previousExpression, setPreviousExpression] = useState<string | null>(null);
  const [result, setResult] = useState<DisplayResult | null>(null);
  const [lastOperation, setLastOperation] = useState<LastBinaryOperation | null>(null);
  const [justEvaluated, setJustEvaluated] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copyMessage, setCopyMessage] = useState<string | null>(null);
  const {
    history,
    memory,
    angleMode,
    answer,
    addHistory,
    clearHistory,
    setMemory,
    setAngleMode,
    setAnswer,
  } = useCalculatorSession();
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    rootRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!copyMessage) {
      return;
    }
    const timer = window.setTimeout(() => setCopyMessage(null), 2500);
    return () => window.clearTimeout(timer);
  }, [copyMessage]);

  const recordHistory = useCallback(
    (expressionText: string, value: number, resultText: string) => {
      addHistory(
        createHistoryEntry({
          expression: expressionText,
          result: resultText,
          value,
          timestamp: Date.now(),
        }),
      );
    },
    [addHistory],
  );

  const copyText = useCallback(async (text: string, successMessage: string) => {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        fallbackCopyText(text);
      }
      setCopyMessage(successMessage);
    } catch {
      try {
        fallbackCopyText(text);
        setCopyMessage(successMessage);
      } catch {
        setCopyMessage('Clipboard access is unavailable. Select the result and copy it manually.');
      }
    }
  }, []);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const appendToken = useCallback(
    (token: string) => {
      const trimmed = token.trim();
      const startsWithBinaryOperator =
        token === '+' ||
        token === '-' ||
        token === '*' ||
        token === '/' ||
        token === '^' ||
        token.startsWith('^') ||
        token === '%' ||
        token === '!' ||
        trimmed === 'mod';

      if (justEvaluated) {
        if (startsWithBinaryOperator) {
          setExpression(trimmed === 'mod' ? 'Ans mod ' : `Ans${token}`);
        } else {
          setExpression(token);
        }
      } else {
        setExpression((current) => `${current}${token}`);
      }
      setJustEvaluated(false);
      clearError();
    },
    [clearError, justEvaluated],
  );

  const clearEntry = useCallback(() => {
    setExpression('');
    setError(null);
    setJustEvaluated(false);
  }, []);

  const allClear = useCallback(() => {
    setExpression('');
    setPreviousExpression(null);
    setResult(null);
    setAnswer(0);
    setLastOperation(null);
    setJustEvaluated(false);
    setError(null);
  }, [setAnswer]);

  const backspace = useCallback(() => {
    setExpression((current) => current.slice(0, -1));
    setJustEvaluated(false);
    setError(null);
  }, []);

  const toggleSign = useCallback(() => {
    if (justEvaluated) {
      setExpression('-Ans');
      setJustEvaluated(false);
      setError(null);
      return;
    }
    setExpression((current) => {
      if (current.startsWith('-')) {
        return current.slice(1);
      }
      return `-${current}`;
    });
    setError(null);
  }, [justEvaluated]);

  const resolveCurrentValue = useCallback((): number => {
    if (justEvaluated && result) {
      return result.value;
    }
    if (expression.trim()) {
      return evaluate(expression, { angleMode, ans: answer }).value;
    }
    return result?.value ?? answer;
  }, [angleMode, answer, expression, justEvaluated, result]);

  const handleMemory = useCallback(
    (operation: MemoryOperation) => {
      if (operation === 'clear') {
        setMemory(null);
        return;
      }
      if (operation === 'recall') {
        if (memory === null) {
          return;
        }
        if (!Number.isFinite(memory)) {
          setMemory(null);
          setError(MEMORY_VALUE_ERROR);
          return;
        }
        appendToken(String(memory));
        return;
      }

      let value: number;
      try {
        value = resolveCurrentValue();
      } catch (memoryError) {
        setError(calculationErrorMessage(memoryError));
        setResult(null);
        setJustEvaluated(false);
        return;
      }

      if (!Number.isFinite(value)) {
        setError(MEMORY_VALUE_ERROR);
        return;
      }

      if (operation === 'store') {
        setMemory(value);
        return;
      }

      const current = memory ?? 0;
      const next = operation === 'add' ? current + value : current - value;
      if (!Number.isFinite(next)) {
        setError(MEMORY_VALUE_ERROR);
        return;
      }
      setMemory(next);
    },
    [appendToken, memory, resolveCurrentValue, setMemory],
  );

  const handleEvaluate = useCallback(() => {
    if (justEvaluated) {
      if (!lastOperation) {
        return;
      }
      try {
        const value = applyRepeatedEquals(answer, lastOperation);
        const formatted = formatNumber(value);
        const operandText = formatNumber(lastOperation.operand).text;
        const repeatExpression = `Ans ${operatorLabel(lastOperation.operator)} ${operandText}`;
        setResult({
          value,
          text: formatted.text,
          approximate: formatted.approximate,
        });
        setAnswer(value);
        setExpression(repeatExpression);
        setPreviousExpression(repeatExpression);
        setError(null);
        setJustEvaluated(true);
        recordHistory(repeatExpression, value, formatted.text);
      } catch (evaluationError) {
        setError(calculationErrorMessage(evaluationError));
        setResult(null);
        setJustEvaluated(false);
      }
      return;
    }

    if (!expression.trim()) {
      return;
    }

    try {
      const evaluated = evaluate(expression, { angleMode, ans: answer });
      const formatted = formatNumber(evaluated.value);
      setResult({
        value: evaluated.value,
        text: formatted.text,
        approximate: formatted.approximate,
      });
      setAnswer(evaluated.value);
      setLastOperation(evaluated.lastBinary);
      setPreviousExpression(expression);
      setError(null);
      setJustEvaluated(true);
      recordHistory(expression, evaluated.value, formatted.text);
    } catch (evaluationError) {
      setError(calculationErrorMessage(evaluationError));
      setResult(null);
      setJustEvaluated(false);
    }
  }, [angleMode, answer, expression, justEvaluated, lastOperation, recordHistory, setAnswer]);

  const handleAction = useCallback(
    (action: CalculatorKeyAction) => {
      switch (action.type) {
        case 'append':
          appendToken(action.value);
          break;
        case 'evaluate':
          handleEvaluate();
          break;
        case 'backspace':
          backspace();
          break;
        case 'clear-entry':
          clearEntry();
          break;
        case 'all-clear':
          allClear();
          break;
        case 'toggle-sign':
          toggleSign();
          break;
        case 'memory':
          handleMemory(action.operation);
          break;
        default:
          break;
      }
    },
    [allClear, appendToken, backspace, clearEntry, handleEvaluate, handleMemory, toggleSign],
  );

  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) {
        return;
      }
      if (isEditableTarget(event.target)) {
        return;
      }
      const target = event.target;
      if (
        target instanceof HTMLElement &&
        target.tagName.toLowerCase() === 'button' &&
        (event.key === 'Enter' || event.key === ' ')
      ) {
        return;
      }

      if (event.key === 'Escape') {
        if (document.querySelector('[role="dialog"][aria-modal="true"]')) {
          return;
        }
        event.preventDefault();
        if (event.shiftKey) {
          allClear();
        } else {
          clearEntry();
        }
        return;
      }
      if (event.key === 'Delete') {
        event.preventDefault();
        allClear();
        return;
      }
      if (event.key === 'Enter' || event.key === '=') {
        event.preventDefault();
        handleEvaluate();
        return;
      }
      if (event.key === 'Backspace') {
        event.preventDefault();
        backspace();
        return;
      }
      if (
        /^[0-9.]$/.test(event.key) ||
        ['+', '-', '*', '/', '^', '%', '!', '(', ')', ','].includes(event.key)
      ) {
        event.preventDefault();
        appendToken(event.key);
      }
    },
    [allClear, appendToken, backspace, clearEntry, handleEvaluate],
  );

  const handleReuseHistory = useCallback((entry: HistoryEntry) => {
    setExpression(entry.expression);
    setJustEvaluated(false);
    setError(null);
  }, []);

  const handleCopyHistory = useCallback(
    (entry: HistoryEntry) => {
      void copyText(entry.result, 'Copied history result.');
    },
    [copyText],
  );

  const handleCopyResult = useCallback(() => {
    if (result) {
      void copyText(result.text, 'Copied result.');
    }
  }, [copyText, result]);

  return (
    <div
      className="page-container v2-calculator-page"
      onKeyDown={handleKeyDown}
      ref={rootRef}
      tabIndex={-1}
    >
      <header className="page-header">
        <p className="eyebrow">Physics Tracker · Calculator</p>
        <h1>Scientific calculator</h1>
        <p className="page-lede">
          A precise general-purpose scientific calculator with memory, previous-answer recall, angle
          modes and session history.
        </p>
      </header>

      <div className="v2-calculator-toolbar">
        <div aria-label="Angle mode" className="v2-calculator-angle" role="group">
          <span className="v2-calculator-toolbar-label">Angle</span>
          <button
            aria-pressed={angleMode === 'deg'}
            className="v2-calculator-angle-button"
            onClick={() => setAngleMode('deg')}
            type="button"
          >
            Degrees
          </button>
          <button
            aria-pressed={angleMode === 'rad'}
            className="v2-calculator-angle-button"
            onClick={() => setAngleMode('rad')}
            type="button"
          >
            Radians
          </button>
        </div>
      </div>

      <div className="v2-calculator-layout">
        <section aria-label="Calculation workspace" className="v2-calculator-main">
          <CalculatorDisplay
            angleMode={angleMode}
            approximate={result?.approximate ?? false}
            error={error}
            expression={expression}
            label="Scientific"
            memory={memory}
            onCopy={handleCopyResult}
            previousExpression={previousExpression}
            resultText={result?.text ?? null}
          />

          <CalculatorKeypad onAction={handleAction} />
        </section>

        <aside aria-label="Calculator history and shortcuts" className="v2-calculator-side">
          <CalculatorHistory
            entries={history}
            onClear={clearHistory}
            onCopy={handleCopyHistory}
            onReuse={handleReuseHistory}
          />
          <details className="v2-concept-card v2-calculator-help">
            <summary>Keyboard shortcuts</summary>
            <dl>
              <div>
                <dt>0–9, .</dt>
                <dd>Enter numbers</dd>
              </div>
              <div>
                <dt>+ − * / ^ % ! ( )</dt>
                <dd>Enter operators and postfix functions</dd>
              </div>
              <div>
                <dt>Enter or =</dt>
                <dd>Evaluate; press again to repeat the last operation</dd>
              </div>
              <div>
                <dt>Backspace</dt>
                <dd>Delete the last character</dd>
              </div>
              <div>
                <dt>Escape</dt>
                <dd>Clear the current entry</dd>
              </div>
              <div>
                <dt>Delete or Shift+Escape</dt>
                <dd>All clear</dd>
              </div>
            </dl>
          </details>
        </aside>
      </div>

      {copyMessage && (
        <p className="v2-study-notice" role="status">
          {copyMessage}
        </p>
      )}
    </div>
  );
}
