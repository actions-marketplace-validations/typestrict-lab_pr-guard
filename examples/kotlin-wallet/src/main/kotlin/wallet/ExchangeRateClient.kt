package wallet

import java.math.BigDecimal
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.time.Duration

fun interface ExchangeRates {
    fun rate(from: String, to: String): BigDecimal
}

/** Fetches rates from the treasury service, which answers with a plain decimal body. */
class ExchangeRateClient(
    private val baseUrl: String,
    private val http: HttpClient = HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(5)).build(),
) : ExchangeRates {
    override fun rate(from: String, to: String): BigDecimal {
        val request = HttpRequest.newBuilder(URI.create("$baseUrl/rates/$from/$to"))
            .timeout(Duration.ofSeconds(5))
            .GET()
            .build()
        val response = http.send(request, HttpResponse.BodyHandlers.ofString())
        check(response.statusCode() == 200) { "rate service returned ${response.statusCode()}" }
        return response.body().trim().toBigDecimal()
    }
}
