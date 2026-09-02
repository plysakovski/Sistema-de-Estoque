import { QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { AppShell } from "./app/app-shell";
import { queryClient } from "./app/query-client";
import "./styles/global.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode><QueryClientProvider client={queryClient}><AppShell /></QueryClientProvider></StrictMode>,
);
