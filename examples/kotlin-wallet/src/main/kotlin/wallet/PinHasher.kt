package wallet

import java.security.MessageDigest
import java.security.SecureRandom
import java.util.Base64
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.PBEKeySpec

/** Hashes wallet PINs with salted PBKDF2, so a leaked table cannot be brute forced cheaply. */
object PinHasher {
    private const val ITERATIONS = 210_000
    private const val KEY_BITS = 256
    private val random = SecureRandom()

    fun hash(pin: String): String {
        val salt = ByteArray(16).also(random::nextBytes)
        val encoder = Base64.getEncoder()
        return encoder.encodeToString(salt) + ":" + encoder.encodeToString(derive(pin, salt))
    }

    fun verify(pin: String, stored: String): Boolean {
        val parts = stored.split(":")
        if (parts.size != 2) return false
        val decoder = Base64.getDecoder()
        val expected = decoder.decode(parts[1])
        return MessageDigest.isEqual(expected, derive(pin, decoder.decode(parts[0])))
    }

    private fun derive(pin: String, salt: ByteArray): ByteArray {
        val spec = PBEKeySpec(pin.toCharArray(), salt, ITERATIONS, KEY_BITS)
        return SecretKeyFactory.getInstance("PBKDF2WithHmacSHA256").generateSecret(spec).encoded
    }
}
