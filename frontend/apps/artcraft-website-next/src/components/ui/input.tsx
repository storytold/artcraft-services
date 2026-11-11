import type { InputHTMLAttributes, ReactNode, Ref, TextareaHTMLAttributes } from "react";
import { twMerge } from "tailwind-merge";
import { Label } from "./label";

// Shared by Input and Textarea.
const FIELD_CLASSES = [
  "w-full rounded-none border border-line bg-bg-raised px-3 py-2.5 text-ink outline-none placeholder:text-faint",
  "transition-colors duration-150 ease-in-out hover:border-line-strong focus:border-ink focus:outline-none",
  "disabled:cursor-not-allowed disabled:opacity-60 disabled:hover:border-line",
];
const FIELD_ERROR_CLASSES = "border-danger focus:border-danger";

/*
 * Ported from @storyteller/ui-input: squared text field on a raised surface
 * with hairline border that sharpens on hover and goes full-ink on focus.
 */
export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  inputClassName?: string;
  label?: string;
  /** Rendered inside the field's left edge (sized by the caller). */
  icon?: ReactNode;
  isError?: boolean;
  errorMessage?: string;
  ref?: Ref<HTMLInputElement>;
}

export function Input({
  label,
  icon,
  inputClassName,
  className,
  id,
  isError,
  errorMessage,
  ref,
  ...rest
}: InputProps) {
  const inputId = id ?? label;
  return (
    <div className={twMerge("flex flex-col", className)}>
      {label && <Label htmlFor={inputId}>{label}</Label>}

      <div className="relative w-full">
        {icon && (
          <span className="pointer-events-none absolute inset-y-0 left-3 flex items-center text-muted">
            {icon}
          </span>
        )}
        <input
          ref={ref}
          id={inputId}
          className={twMerge(
            FIELD_CLASSES,
            "h-10",
            icon && "pl-10",
            isError && FIELD_ERROR_CLASSES,
            inputClassName,
          )}
          {...rest}
        />
        <FieldError message={errorMessage} />
      </div>
    </div>
  );
}

export interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  textareaClassName?: string;
  label?: string;
  isError?: boolean;
  errorMessage?: string;
  ref?: Ref<HTMLTextAreaElement>;
}

/** Multi-line twin of Input: same field styling, grows by user resize. */
export function Textarea({
  label,
  textareaClassName,
  className,
  id,
  isError,
  errorMessage,
  ref,
  rows = 4,
  ...rest
}: TextareaProps) {
  const textareaId = id ?? label;
  return (
    <div className={twMerge("flex flex-col", className)}>
      {label && <Label htmlFor={textareaId}>{label}</Label>}
      <textarea
        ref={ref}
        id={textareaId}
        rows={rows}
        className={twMerge(
          FIELD_CLASSES,
          "block resize-y",
          isError && FIELD_ERROR_CLASSES,
          textareaClassName,
        )}
        {...rest}
      />
      <FieldError message={errorMessage} />
    </div>
  );
}

function FieldError({ message }: { message?: string }) {
  if (!message) return null;
  return (
    <p className="mt-1 font-mono text-[10px] uppercase tracking-[0.1em] text-danger">
      {message}
    </p>
  );
}

export default Input;
