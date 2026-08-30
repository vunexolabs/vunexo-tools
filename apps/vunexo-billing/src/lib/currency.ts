// Static reference data — country → default currency, currency → symbol and
// decimal places (ISO 4217 minor-unit exponent). Not user data, so it lives
// as a frontend constant rather than a database table; extend the lists
// below if a country/currency is missing rather than special-casing one
// somewhere else.

export interface CurrencyMeta {
  name: string;
  symbol: string;
  /** ISO 4217 minor-unit exponent — how many digits after the decimal point this currency's minor unit implies. */
  decimals: number;
}

export const CURRENCIES: Record<string, CurrencyMeta> = {
  INR: { name: "Indian Rupee", symbol: "₹", decimals: 2 },
  USD: { name: "US Dollar", symbol: "$", decimals: 2 },
  EUR: { name: "Euro", symbol: "€", decimals: 2 },
  GBP: { name: "British Pound", symbol: "£", decimals: 2 },
  AUD: { name: "Australian Dollar", symbol: "A$", decimals: 2 },
  CAD: { name: "Canadian Dollar", symbol: "C$", decimals: 2 },
  NZD: { name: "New Zealand Dollar", symbol: "NZ$", decimals: 2 },
  SGD: { name: "Singapore Dollar", symbol: "S$", decimals: 2 },
  HKD: { name: "Hong Kong Dollar", symbol: "HK$", decimals: 2 },
  AED: { name: "UAE Dirham", symbol: "د.إ", decimals: 2 },
  SAR: { name: "Saudi Riyal", symbol: "﷼", decimals: 2 },
  QAR: { name: "Qatari Riyal", symbol: "ر.ق", decimals: 2 },
  BHD: { name: "Bahraini Dinar", symbol: ".د.ب", decimals: 3 },
  KWD: { name: "Kuwaiti Dinar", symbol: "د.ك", decimals: 3 },
  OMR: { name: "Omani Rial", symbol: "ر.ع.", decimals: 3 },
  JOD: { name: "Jordanian Dinar", symbol: "د.ا", decimals: 3 },
  TND: { name: "Tunisian Dinar", symbol: "د.ت", decimals: 3 },
  JPY: { name: "Japanese Yen", symbol: "¥", decimals: 0 },
  KRW: { name: "South Korean Won", symbol: "₩", decimals: 0 },
  VND: { name: "Vietnamese Dong", symbol: "₫", decimals: 0 },
  IDR: { name: "Indonesian Rupiah", symbol: "Rp", decimals: 0 },
  CLP: { name: "Chilean Peso", symbol: "$", decimals: 0 },
  ISK: { name: "Icelandic Krona", symbol: "kr", decimals: 0 },
  UGX: { name: "Ugandan Shilling", symbol: "USh", decimals: 0 },
  PYG: { name: "Paraguayan Guarani", symbol: "₲", decimals: 0 },
  CNY: { name: "Chinese Yuan", symbol: "¥", decimals: 2 },
  CHF: { name: "Swiss Franc", symbol: "CHF", decimals: 2 },
  SEK: { name: "Swedish Krona", symbol: "kr", decimals: 2 },
  NOK: { name: "Norwegian Krone", symbol: "kr", decimals: 2 },
  DKK: { name: "Danish Krone", symbol: "kr", decimals: 2 },
  PLN: { name: "Polish Zloty", symbol: "zł", decimals: 2 },
  CZK: { name: "Czech Koruna", symbol: "Kč", decimals: 2 },
  HUF: { name: "Hungarian Forint", symbol: "Ft", decimals: 2 },
  RON: { name: "Romanian Leu", symbol: "lei", decimals: 2 },
  TRY: { name: "Turkish Lira", symbol: "₺", decimals: 2 },
  RUB: { name: "Russian Ruble", symbol: "₽", decimals: 2 },
  ZAR: { name: "South African Rand", symbol: "R", decimals: 2 },
  NGN: { name: "Nigerian Naira", symbol: "₦", decimals: 2 },
  EGP: { name: "Egyptian Pound", symbol: "£", decimals: 2 },
  KES: { name: "Kenyan Shilling", symbol: "KSh", decimals: 2 },
  GHS: { name: "Ghanaian Cedi", symbol: "₵", decimals: 2 },
  PKR: { name: "Pakistani Rupee", symbol: "₨", decimals: 2 },
  BDT: { name: "Bangladeshi Taka", symbol: "৳", decimals: 2 },
  LKR: { name: "Sri Lankan Rupee", symbol: "₨", decimals: 2 },
  NPR: { name: "Nepalese Rupee", symbol: "₨", decimals: 2 },
  MMK: { name: "Myanmar Kyat", symbol: "K", decimals: 2 },
  THB: { name: "Thai Baht", symbol: "฿", decimals: 2 },
  MYR: { name: "Malaysian Ringgit", symbol: "RM", decimals: 2 },
  PHP: { name: "Philippine Peso", symbol: "₱", decimals: 2 },
  MXN: { name: "Mexican Peso", symbol: "$", decimals: 2 },
  BRL: { name: "Brazilian Real", symbol: "R$", decimals: 2 },
  ARS: { name: "Argentine Peso", symbol: "$", decimals: 2 },
  COP: { name: "Colombian Peso", symbol: "$", decimals: 2 },
  PEN: { name: "Peruvian Sol", symbol: "S/", decimals: 2 },
  ILS: { name: "Israeli New Shekel", symbol: "₪", decimals: 2 },
};

export interface CountryMeta {
  code: string;
  name: string;
  currencyCode: string;
}

export const COUNTRIES: CountryMeta[] = [
  { code: "IN", name: "India", currencyCode: "INR" },
  { code: "US", name: "United States", currencyCode: "USD" },
  { code: "GB", name: "United Kingdom", currencyCode: "GBP" },
  { code: "AU", name: "Australia", currencyCode: "AUD" },
  { code: "CA", name: "Canada", currencyCode: "CAD" },
  { code: "NZ", name: "New Zealand", currencyCode: "NZD" },
  { code: "SG", name: "Singapore", currencyCode: "SGD" },
  { code: "HK", name: "Hong Kong", currencyCode: "HKD" },
  { code: "AE", name: "United Arab Emirates", currencyCode: "AED" },
  { code: "SA", name: "Saudi Arabia", currencyCode: "SAR" },
  { code: "QA", name: "Qatar", currencyCode: "QAR" },
  { code: "BH", name: "Bahrain", currencyCode: "BHD" },
  { code: "KW", name: "Kuwait", currencyCode: "KWD" },
  { code: "OM", name: "Oman", currencyCode: "OMR" },
  { code: "JO", name: "Jordan", currencyCode: "JOD" },
  { code: "TN", name: "Tunisia", currencyCode: "TND" },
  { code: "JP", name: "Japan", currencyCode: "JPY" },
  { code: "KR", name: "South Korea", currencyCode: "KRW" },
  { code: "VN", name: "Vietnam", currencyCode: "VND" },
  { code: "ID", name: "Indonesia", currencyCode: "IDR" },
  { code: "CL", name: "Chile", currencyCode: "CLP" },
  { code: "IS", name: "Iceland", currencyCode: "ISK" },
  { code: "UG", name: "Uganda", currencyCode: "UGX" },
  { code: "PY", name: "Paraguay", currencyCode: "PYG" },
  { code: "CN", name: "China", currencyCode: "CNY" },
  { code: "CH", name: "Switzerland", currencyCode: "CHF" },
  { code: "SE", name: "Sweden", currencyCode: "SEK" },
  { code: "NO", name: "Norway", currencyCode: "NOK" },
  { code: "DK", name: "Denmark", currencyCode: "DKK" },
  { code: "PL", name: "Poland", currencyCode: "PLN" },
  { code: "CZ", name: "Czech Republic", currencyCode: "CZK" },
  { code: "HU", name: "Hungary", currencyCode: "HUF" },
  { code: "RO", name: "Romania", currencyCode: "RON" },
  { code: "TR", name: "Turkey", currencyCode: "TRY" },
  { code: "RU", name: "Russia", currencyCode: "RUB" },
  { code: "ZA", name: "South Africa", currencyCode: "ZAR" },
  { code: "NG", name: "Nigeria", currencyCode: "NGN" },
  { code: "EG", name: "Egypt", currencyCode: "EGP" },
  { code: "KE", name: "Kenya", currencyCode: "KES" },
  { code: "GH", name: "Ghana", currencyCode: "GHS" },
  { code: "PK", name: "Pakistan", currencyCode: "PKR" },
  { code: "BD", name: "Bangladesh", currencyCode: "BDT" },
  { code: "LK", name: "Sri Lanka", currencyCode: "LKR" },
  { code: "NP", name: "Nepal", currencyCode: "NPR" },
  { code: "MM", name: "Myanmar", currencyCode: "MMK" },
  { code: "TH", name: "Thailand", currencyCode: "THB" },
  { code: "MY", name: "Malaysia", currencyCode: "MYR" },
  { code: "PH", name: "Philippines", currencyCode: "PHP" },
  { code: "MX", name: "Mexico", currencyCode: "MXN" },
  { code: "BR", name: "Brazil", currencyCode: "BRL" },
  { code: "AR", name: "Argentina", currencyCode: "ARS" },
  { code: "CO", name: "Colombia", currencyCode: "COP" },
  { code: "PE", name: "Peru", currencyCode: "PEN" },
  { code: "IL", name: "Israel", currencyCode: "ILS" },
  { code: "DE", name: "Germany", currencyCode: "EUR" },
  { code: "FR", name: "France", currencyCode: "EUR" },
  { code: "ES", name: "Spain", currencyCode: "EUR" },
  { code: "IT", name: "Italy", currencyCode: "EUR" },
  { code: "NL", name: "Netherlands", currencyCode: "EUR" },
  { code: "IE", name: "Ireland", currencyCode: "EUR" },
  { code: "PT", name: "Portugal", currencyCode: "EUR" },
];

export function currencyMeta(currencyCode: string): CurrencyMeta {
  return CURRENCIES[currencyCode] ?? { name: currencyCode, symbol: currencyCode, decimals: 2 };
}
