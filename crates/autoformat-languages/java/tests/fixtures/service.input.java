package com.example.orders;

import java.io.IOException;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;

/**
 * Keeps orders in memory and answers questions about them.
 *
 * <p>Not safe to share between programs.
 */
@Service
public class OrderService implements AutoCloseable
{
    private static final Duration TIMEOUT=Duration.ofSeconds( 30 );
    private static final String QUERY = """
            SELECT id,  total
              FROM orders
             WHERE customer = ?
            """;

    private final Map<String,List<Order>> ordersByCustomer=new ConcurrentHashMap<>();
    private final OrderRepository repository;
    private int saved;   // how many were saved

    public OrderService(OrderRepository repository)
    {
        this.repository=repository;
    }

    /** Adds an order and returns its number. */
    public long add(String customer,Order order) throws IOException
    {
        if(customer==null||customer.isBlank())
        {
            throw new IllegalArgumentException("customer is required");
        }
        List<Order> orders=ordersByCustomer.computeIfAbsent(customer,key->new ArrayList<>());
        synchronized(orders)
        {
            orders.add(order);
        }
        try
        {
            repository.save(order);
            saved++;
        }
        catch(IOException error)
        {
            orders.remove(order);
            throw error;
        }
        finally
        {
            repository.flush();
        }
        return order.id();
    }

    public Optional<Order> largest(String customer)
    {
        return ordersByCustomer.getOrDefault(customer,List.of()).stream()
            .filter(order->order.total()>0)
            .max((first,second)->Double.compare(first.total(),second.total()));
    }

    public Map<Status,Long> countByStatus()
    {
        return ordersByCustomer.values().stream().flatMap(List::stream)
                .collect(Collectors.groupingBy(Order::status,Collectors.counting()));
    }

    public String describe(Order order)
    {
        String size=order.total()>1000?"large":order.total()>100?"medium":"small";
        return switch(order.status())
        {
            case NEW,PAID->size+" order waiting";
            case SHIPPED->
            {
                String when=order.shippedAt().toString();
                yield size+" order sent "+when;
            }
            default->"unknown";
        };
    }

    public double total(String customer)
    {
        double sum=0;
        for(Order order:ordersByCustomer.getOrDefault(customer,List.of()))
        {
            switch(order.status())
            {
                case CANCELLED:
                    continue;
                case NEW:
                case PAID:
                    sum+=order.total();
                    break;
                default:
                    sum+=order.total()*0.5;
            }
        }
        return sum;
    }

    @Override
    public void close()
    {
        ordersByCustomer.clear();
    }

    public enum Status { NEW, PAID, SHIPPED, CANCELLED }

    public record Order(long id,double total,Status status,java.time.Instant shippedAt) {}
}
