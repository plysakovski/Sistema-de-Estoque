export const dateTimeFormatter = new Intl.DateTimeFormat("pt-BR", {
  dateStyle: "short",
  timeStyle: "short",
});

export function formatDateTime(value: string): string {
  return dateTimeFormatter.format(new Date(value));
}
export function movementLabel(kind: string): string {
  return { entry: "Entrada", exit: "Saída", transfer: "Transferência", adjustment: "Ajuste" }[kind] ?? kind;
}
