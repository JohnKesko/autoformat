package shop.orders

import java.time.Instant
import kotlin.collections.List as KList

/**
 * Keeps the orders of one customer.
 *
 * @param T the kind of item
 */
sealed interface Event {
    data class Created(val id: Long, val at: Instant = Instant.now()) : Event
    data object Closed : Event
}

/* outer /* nested */ still a comment */
abstract class OrderService<in K : Comparable<K>, out V>(
    private val store: Store<K, V>,
    private val clock: Clock = Clock.system()
) : Service(), AutoCloseable {
    private val listeners = mutableListOf<(Event) -> Unit>()
    @Volatile private var closed = false
    val size: Int get() = store.size

    var limit: Int = 100
        set(value) {
            require(value > 0) { "limit must be positive, was $value" }
            field = value
        }

    @Throws(IllegalStateException::class)
    @JvmOverloads
    fun find(key: K, fallback: V? = null, vararg tags: String): V? {
        check(!closed) { "closed" }
        val found = store[key] ?: fallback
        return found?.takeIf { tags.isEmpty() || it.toString() in tags }
    }

    fun total(prices: Map<String, Double>, discount: Double = 0.0): Double = prices.values.sum() * (1 - discount)

    suspend fun <R> each(action: suspend (K, V) -> R): KList<R> {
        val results = ArrayList<R>()
        outer@ for ((key, value) in store.entries) {
            for (listener in listeners)
                listener(Event.Created(key.hashCode().toLong()))
            if (value == null) continue@outer
            results += action(key, value)
        }
        return results
    }

    fun describe(event: Event): String = when (event) {
        is Event.Created -> "created ${event.id} at ${event.at}"
        Event.Closed,
        null -> "closed"
    }

    override fun close() {
        try {
            store.flush()
        } catch (error: java.io.IOException) {
            log(error)
        } finally {
            closed = true
        }
    }

    companion object {
        const val VERSION = "1.0"
        @JvmStatic fun <K : Comparable<K>, V> empty(): OrderService<K, V>? = null
    }
}

fun Int.isEven() = this % 2 == 0

val String.shout: String
    get() = uppercase() + "!"

typealias Listener<T> = (index: Int, item: T) -> Boolean
inline fun <reified T> Any?.isA() = this is T
