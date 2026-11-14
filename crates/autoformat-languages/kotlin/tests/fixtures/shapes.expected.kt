@file:JvmName("Demo")
package demo.app

import kotlin.math.*

/**
 * KDoc for [User].
 */
@Suppress("unused") @Deprecated("x")
data class User<out T : Any>(val name: String, var age: Int = 0) : Base<T>(name), Comparable<User<*>> where T : Comparable<T> {
    @get:Rule val rule = 1

    var size: Int = 0
        get() = field
        private set(value) { field = value }

    companion object { const val MAX = 10 }
    init { require(age >= 0) { "bad $age ${name.length}" } }
    constructor(name: String) : this(name, 0)
    override fun compareTo(other: User<*>): Int = age - other.age
    fun <R> String?.ext(block: (Int, String) -> R, vararg xs: Int): R? = null
}

enum class Color(val rgb: Int) { RED(0xFF0000), GREEN(0x00FF00) { override fun f() = 1 }; abstract fun f(): Int }

fun main(args: Array<String>) {
    val (a, b) = Pair(1, 2)
    val s = """raw
      $a  text"""
    val r = 0..10
    for (i in 1 until 10 step 2) print(i)
    loop@ for (x in r) { if (x !in q) continue@loop else break@loop }
    val y = x as? String ?: return
    val z = y!!.length + -a * b
    val f = ::main; val g = String::length
    list.filter { it > 0 }.map { v -> v * 2 }.forEach(::println)
    foo(1, name = 2, *arr) { x, y -> x + y }
    val o = object : Runnable { override fun run() {} }
    when (x) {
        1, 2 -> a()
        in 3..4 -> b()
        is String -> c()
        !is Int -> d()
        else -> { e() }
    }
    try { a() } catch (e: Exception) { b() } finally { c() }
    do { i-- } while (i > 0)
    val t = if (a > b) a else b
    val n: Int? = null
    x?.let { it.foo() }
    arr[0] = arr[1]
    if (a) {
        b()
    } else {
        c()
    }
    val h = listOf(1, 2, 3)
        .map { it + 1 }
        .filter {
            it > 2
        }
    return@main
}
