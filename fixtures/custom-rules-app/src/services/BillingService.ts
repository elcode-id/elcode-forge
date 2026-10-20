export class BillingService {
  charge(amount: number): boolean {
    return amount > 0;
  }
}
