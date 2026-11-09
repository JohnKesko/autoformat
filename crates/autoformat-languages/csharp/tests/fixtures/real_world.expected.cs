// Order processing for a small shop.
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace Shop.Orders;

public enum OrderStatus { Pending, Paid, Shipped, Cancelled }

public sealed class Order {
    public Guid Id { get; init; } = Guid.NewGuid();
    public string Customer { get; set; } = string.Empty;
    public List<OrderLine> Lines { get; } = new();
    public OrderStatus Status { get; private set; } = OrderStatus.Pending;

    public decimal Total => Lines.Sum(line => line.Price * line.Quantity);

    public void MarkPaid() {
        if (Status != OrderStatus.Pending)
            throw new InvalidOperationException($"Order {Id} cannot be paid in state {Status}.");
        Status = OrderStatus.Paid;
    }
}

public record OrderLine(string Product, decimal Price, int Quantity);

public class OrderService {
    private readonly IDictionary<Guid, Order> _orders = new Dictionary<Guid, Order>();

    /// <summary>Creates an order from a shopping cart.</summary>
    /// <param name="customer">The customer name.</param>
    public async Task<Order> CreateAsync(string customer, IEnumerable<(string Product, decimal Price, int Quantity)> cart) {
        var order = new Order { Customer = customer };
        foreach (var (product, price, quantity) in cart) {
            if (quantity <= 0) continue;
            order.Lines.Add(new OrderLine(product, price, quantity));
        }
        await Task.Delay(1).ConfigureAwait(false);
        _orders[order.Id] = order;
        return order;
    }

    public IReadOnlyList<Order> FindByStatus(OrderStatus status) =>
        _orders.Values.Where(order => order.Status == status).OrderBy(order => order.Customer).ToList();

    public string Describe(Order order) => order.Status switch
    {
        OrderStatus.Pending => "waiting for payment",
        OrderStatus.Paid or OrderStatus.Shipped => "on its way",
        _ => "cancelled",
    };
}
