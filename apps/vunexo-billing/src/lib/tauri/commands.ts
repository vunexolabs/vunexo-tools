// Typed call signatures for Tauri commands. Add one entry here per command
// exposed in src-tauri/src/commands/mod.rs.
import { callCommand } from "./client";
import type {
  Business,
  Customer,
  CustomerFields,
  CustomerFilter,
  CustomerListItem,
} from "./types";

/** Round 1 technical spike: proves the React -> Tauri -> Rust round trip. */
export function greet(name: string): Promise<string> {
  return callCommand<string>("greet", { name });
}

export function createBusiness(business: Business): Promise<Business> {
  return callCommand<Business>("create_business", { business });
}

export function getBusiness(): Promise<Business | null> {
  return callCommand<Business | null>("get_business");
}

export function updateBusiness(business: Business): Promise<Business> {
  return callCommand<Business>("update_business", { business });
}

export function createCustomer(fields: CustomerFields): Promise<Customer> {
  return callCommand<Customer>("create_customer", { fields });
}

export function updateCustomer(id: number, fields: CustomerFields): Promise<Customer> {
  return callCommand<Customer>("update_customer", { id, fields });
}

export function archiveCustomer(id: number): Promise<void> {
  return callCommand<void>("archive_customer", { id });
}

export function restoreCustomer(id: number): Promise<void> {
  return callCommand<void>("restore_customer", { id });
}

export function deleteCustomer(id: number): Promise<void> {
  return callCommand<void>("delete_customer", { id });
}

export function getCustomer(id: number): Promise<Customer> {
  return callCommand<Customer>("get_customer", { id });
}

export function listCustomers(filter: CustomerFilter): Promise<CustomerListItem[]> {
  return callCommand<CustomerListItem[]>("list_customers", { filter });
}
