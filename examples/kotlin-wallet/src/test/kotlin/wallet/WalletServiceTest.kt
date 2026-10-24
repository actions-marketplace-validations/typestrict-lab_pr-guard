package wallet

import java.math.BigDecimal
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class WalletServiceTest {
    private val service = WalletService { _, _ -> BigDecimal("0.92") }

    private fun twoWallets(toCurrency: String = "USD") {
        service.open("a", "alice", "USD", "1234")
        service.open("b", "bob", toCurrency, "5678")
        service.deposit("a", 10_00)
    }

    @Test
    fun `transfer moves money between wallets of the same currency`() {
        twoWallets()
        assertEquals(4_00L, service.transfer("alice", "a", "b", 4_00, "1234"))
        assertEquals(6_00L, service.get("a").balanceCents)
        assertEquals(4_00L, service.get("b").balanceCents)
    }

    @Test
    fun `transfer converts currency`() {
        twoWallets(toCurrency = "EUR")
        // 1.25 USD * 0.92 = 1.15 EUR exactly; 0.25 USD * 0.92 = 0.23 EUR
        assertEquals(1_15L, service.transfer("alice", "a", "b", 1_25, "1234"))
        assertEquals(23L, service.transfer("alice", "a", "b", 25, "1234"))
    }

    @Test
    fun `only the owner with the right PIN can transfer`() {
        twoWallets()
        assertFailsWith<Forbidden> { service.transfer("bob", "a", "b", 1_00, "1234") }
        assertFailsWith<Forbidden> { service.transfer("alice", "a", "b", 1_00, "9999") }
    }

    @Test
    fun `cannot overdraw`() {
        twoWallets()
        assertFailsWith<InsufficientFunds> { service.transfer("alice", "a", "b", 10_01, "1234") }
    }
}
