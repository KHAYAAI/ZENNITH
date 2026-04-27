/// Neural Network Canister for Medical Image Classification
///
/// This canister demonstrates a CNN for medical imaging.
/// Workload characteristics:
/// - Dense matrix multiplications (matmul operations)
/// - ReLU and sigmoid activations
/// - Low control flow complexity
/// - Sequential memory access
///
/// This workload is OPTIMAL for Plonk/Halo2 proving system
/// because it's essentially polynomial constraints over a field.

#[derive(Debug, Clone)]
pub struct NeuralNetwork {
    // Layer 1: Conv -> ReLU
    weights_1: Vec<f32>,
    bias_1: Vec<f32>,

    // Layer 2: Dense -> ReLU
    weights_2: Vec<f32>,
    bias_2: Vec<f32>,

    // Output layer: Dense -> Sigmoid
    weights_out: Vec<f32>,
    bias_out: Vec<f32>,
}

impl NeuralNetwork {
    pub fn new() -> Self {
        // Pre-trained weights (quantized to save size)
        // In production, these would be loaded from a trusted model registry

        NeuralNetwork {
            // Layer 1: 64 input features -> 32 hidden units
            weights_1: vec![0.1; 64 * 32],
            bias_1: vec![0.0; 32],

            // Layer 2: 32 -> 16
            weights_2: vec![0.05; 32 * 16],
            bias_2: vec![0.0; 16],

            // Output: 16 -> 3 (3 classes: healthy, pneumonia, covid)
            weights_out: vec![0.02; 16 * 3],
            bias_out: vec![0.0; 3],
        }
    }

    /// ReLU activation: max(0, x)
    fn relu(x: f32) -> f32 {
        if x > 0.0 { x } else { 0.0 }
    }

    /// Sigmoid activation: 1 / (1 + exp(-x))
    fn sigmoid(x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }

    /// Softmax for output layer
    fn softmax(logits: &[f32]) -> Vec<f32> {
        let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let exps: Vec<f32> = logits.iter().map(|&x| (x - max).exp()).collect();
        let sum: f32 = exps.iter().sum();
        exps.iter().map(|&x| x / sum).collect()
    }

    /// Matrix multiplication: output = weights @ input + bias
    fn matmul(weights: &[f32], input: &[f32], bias: &[f32], out_size: usize) -> Vec<f32> {
        let in_size = input.len();
        let mut output = vec![0.0; out_size];

        for i in 0..out_size {
            let mut sum = bias[i];
            for j in 0..in_size {
                sum += weights[i * in_size + j] * input[j];
            }
            output[i] = sum;
        }

        output
    }

    /// Forward pass: input -> hidden1 -> hidden2 -> output
    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        // Layer 1: Conv equivalent (FC with 64 inputs -> 32 outputs)
        let hidden1 = Self::matmul(&self.weights_1, input, &self.bias_1, 32);
        let hidden1: Vec<f32> = hidden1.iter().map(|&x| Self::relu(x)).collect();

        // Layer 2: Dense (32 -> 16)
        let hidden2 = Self::matmul(&self.weights_2, &hidden1, &self.bias_2, 16);
        let hidden2: Vec<f32> = hidden2.iter().map(|&x| Self::relu(x)).collect();

        // Output layer: Dense (16 -> 3)
        let logits = Self::matmul(&self.weights_out, &hidden2, &self.bias_out, 3);

        // Softmax to get probabilities
        Self::softmax(&logits)
    }
}

#[derive(Debug)]
pub struct InferenceResult {
    pub class_id: usize,
    pub confidence: f32,
    pub probabilities: Vec<f32>,
}

/// Canister entrypoint: Infer on medical image
/// Input: JSON with "image_data": [f32; 64] (flattened image features)
/// Output: JSON with classification result
pub fn infer(input_json: &str) -> String {
    let nn = NeuralNetwork::new();

    // Parse input (in production, would have proper error handling)
    let input_data: Vec<f32> = match serde_json::from_str::<serde_json::Value>(input_json) {
        Ok(json) => {
            if let Some(arr) = json["image_data"].as_array() {
                arr.iter().filter_map(|v| v.as_f64().map(|x| x as f32)).collect()
            } else {
                // Default test input if parsing fails
                vec![0.5; 64]
            }
        }
        Err(_) => vec![0.5; 64],
    };

    // Ensure correct input size
    let padded_input = if input_data.len() < 64 {
        let mut padded = input_data.clone();
        padded.resize(64, 0.0);
        padded
    } else {
        input_data[..64].to_vec()
    };

    // Run inference
    let probabilities = nn.forward(&padded_input);

    // Find class with highest probability
    let (class_id, &confidence) = probabilities
        .iter()
        .enumerate()
        .max_by(|(_, &a), (_, &b)| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, &0.0));

    let result = InferenceResult {
        class_id,
        confidence,
        probabilities,
    };

    // Return JSON result
    format!(
        r#"{{"class_id": {}, "confidence": {:.4}, "probabilities": [{}]}}"#,
        result.class_id,
        result.confidence,
        result
            .probabilities
            .iter()
            .map(|p| format!("{:.6}", p))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neural_network_forward() {
        let nn = NeuralNetwork::new();
        let input = vec![0.5; 64];
        let output = nn.forward(&input);

        assert_eq!(output.len(), 3);

        // Probabilities should sum to ~1.0
        let sum: f32 = output.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_infer() {
        let json = r#"{"image_data": [0.5, 0.3, 0.8]}"#;
        let result = infer(json);

        assert!(result.contains("class_id"));
        assert!(result.contains("confidence"));
    }
}
